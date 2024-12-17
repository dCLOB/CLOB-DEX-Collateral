use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::{
    error::Error,
    order::{AddField, NewOrder, Order, OrderSide, OrderType},
    orderbook::{OrderBook, OrderBookId},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[contracttype]
pub enum FillStatus {
    Complete,
    Partial,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[contracttype]
pub struct MakerFill {
    pub oix: OrderBookId,
    pub maker_order: Order,
    pub fill_type: FillStatus,
    pub fill_amount: u128,
}

#[contracttype]
pub struct PendingFill {
    pub taker_order: NewOrder,
    pub maker_fills: Vec<MakerFill>,
    pub taker_fill_status: FillStatus,
}

impl PendingFill {
    pub fn new(
        taker_order: NewOrder,
        maker_fills: Vec<MakerFill>,
        taker_fill_status: FillStatus,
    ) -> Self {
        Self {
            taker_order,
            maker_fills,
            taker_fill_status,
        }
    }
}

fn fill_order(
    env: &Env,
    orderbook: &OrderBook,
    taker: NewOrder,
    side: OrderSide,
    order_type: OrderType,
) -> Result<PendingFill, Error> {
    let mut maker_fills = soroban_sdk::vec![env];
    let mut taker_fill_status = FillStatus::None;
    let mut taker_rem_q = taker.quantity;

    let maker_side = match side {
        OrderSide::BUY => OrderSide::SELL,
        OrderSide::SELL => OrderSide::BUY,
    };

    for (oix, order) in orderbook.maker_orders_iter(maker_side) {
        if order_type == OrderType::Limit
            && ((side == OrderSide::BUY && order.price > taker.price)
                || (side == OrderSide::SELL && order.price < taker.price))
        {
            continue; // Skip orders that don't meet the price condition for limit orders
        }

        let fill_amount = if order.quantity <= taker_rem_q {
            order.quantity
        } else {
            taker_rem_q
        };

        let fill_type = if fill_amount == order.quantity {
            FillStatus::Complete
        } else {
            FillStatus::Partial
        };

        maker_fills.push_back(MakerFill {
            oix,
            maker_order: order,
            fill_type,
            fill_amount,
        });

        if taker_rem_q == fill_amount {
            taker_fill_status = FillStatus::Complete;
            taker_rem_q = 0;
            break;
        } else {
            taker_fill_status = FillStatus::Partial;
            taker_rem_q -= fill_amount;
        }
    }

    if taker_rem_q == taker.quantity {
        taker_fill_status = FillStatus::None;
    }

    let pending_fill = PendingFill::new(taker, maker_fills, taker_fill_status);

    Ok(pending_fill)
}

fn finalize_matching(
    env: &Env,
    order_book: &mut OrderBook,
    pending_fill: PendingFill,
) -> Result<(FillStatus, Option<NewOrder>, Vec<Order>), Error> {
    let mut taker_order_remaining_quantity = pending_fill.taker_order.quantity;

    for fill in pending_fill.maker_fills.iter() {
        if order_book.try_get(fill.oix.clone()).is_err() {
            return Err(Error::InvalidOrderId);
        }
    }

    let mut maker_orders = soroban_sdk::vec![env];

    for MakerFill {
        oix,
        maker_order: order,
        fill_type,
        ..
    } in pending_fill.maker_fills
    {
        match fill_type {
            // complete fill for a maker order.
            FillStatus::Complete => {
                let maker_order = order_book.remove_order(oix)?; // this should never fail because we already checked that the order exists.
                assert_eq!(maker_order, order);

                taker_order_remaining_quantity -= maker_order.quantity; // if this also filled the taker order, then we wont loop again.

                maker_orders.push_back(maker_order);
            }
            // partial fill for a maker order also means a complete fill for the taker order.
            FillStatus::Partial => {
                let mut maker_order = order_book.try_get(oix.clone())?; // this should never fail because we already checked that the order exists.
                assert_eq!(maker_order, order);
                assert!(taker_order_remaining_quantity < maker_order.quantity);
                maker_order.quantity -= taker_order_remaining_quantity;

                order_book
                    .update_order(oix, maker_order.clone())
                    .ok_or(Error::InvalidIdFailedToUpdate)?;

                let mut maker_order_fill = order;
                maker_order_fill.quantity -= maker_order.quantity;

                maker_orders.push_back(maker_order_fill);
                taker_order_remaining_quantity = 0;
            }
            FillStatus::None => unreachable!(),
        }
    }

    match pending_fill.taker_fill_status {
        FillStatus::Complete => assert_eq!(taker_order_remaining_quantity, 0),
        FillStatus::Partial => {
            assert!(pending_fill.taker_order.quantity > taker_order_remaining_quantity);
        }
        FillStatus::None => assert_eq!(
            taker_order_remaining_quantity,
            pending_fill.taker_order.quantity
        ),
    }

    let taker_order = if taker_order_remaining_quantity > 0 {
        let mut taker_order = pending_fill.taker_order;
        taker_order.quantity = taker_order_remaining_quantity;
        Some(taker_order)
    } else {
        // the taker order was completely filled.
        None
    };

    Ok((pending_fill.taker_fill_status, taker_order, maker_orders))
}

pub fn place_order(
    env: &Env,
    order_book: &mut OrderBook,
    order_type: OrderType,
    side: OrderSide,
    order: NewOrder,
    account: Address,
) -> Result<(Option<(OrderBookId, NewOrder)>, Vec<Order>), Error> {
    let pending_fill = fill_order(env, order_book, order, side, order_type)?;

    let (_, taker_order, maker_orders) = finalize_matching(env, order_book, pending_fill)?;

    if let Some(order) = taker_order {
        let new_account_order = order.clone().into_order(account);
        // order was not completely filled, add it to the orderbook.
        // Handle this as a GoodTillCancel time force
        let order_id = match side {
            OrderSide::BUY => order_book.add_buy_order(new_account_order, env),
            OrderSide::SELL => order_book.add_sell_order(new_account_order, env),
        };

        Ok((Some((order_id, order)), maker_orders))
    } else {
        Ok((None, maker_orders))
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use crate::order::{NewOrder, OrderSide, OrderType};
    use crate::orderbook::{OrderBook, OrderBookId, PriceLevelId};
    use crate::trading_ops::place_order;
    use soroban_sdk::Env;
    use soroban_sdk::{testutils::Address as AddressTestTrait, Address};

    #[test]
    fn test_place_limit_buy_order() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        let new_order = NewOrder {
            quantity: 100,
            price: 50,
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };
        let account = Address::generate(&env);

        let result = place_order(
            &env,
            &mut order_book,
            OrderType::Limit,
            OrderSide::BUY,
            new_order.clone(),
            account.clone(),
        );

        assert!(result.is_ok());
        let (placed_order, matched_orders) = result.unwrap();
        assert!(matched_orders.is_empty());

        let (order_id, res_new_order) = placed_order.unwrap();

        assert_eq!(new_order, res_new_order);
        assert_eq!(
            order_id,
            OrderBookId::BuyId(PriceLevelId { id: 1, price: 50 })
        );
    }

    #[test]
    fn test_place_market_sell_order() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        let new_order = NewOrder {
            quantity: 50,
            price: 100,
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };
        let account = Address::generate(&env);

        let result = place_order(
            &env,
            &mut order_book,
            OrderType::Market,
            OrderSide::SELL,
            new_order.clone(),
            account.clone(),
        );

        assert!(result.is_ok());

        let (placed_order, matched_orders) = result.unwrap();
        assert!(matched_orders.is_empty());

        let (order_id, res_new_order) = placed_order.unwrap();

        assert_eq!(new_order, res_new_order);
        assert_eq!(
            order_id,
            OrderBookId::SellId(PriceLevelId { id: 1, price: 100 })
        );
    }

    #[test]
    fn test_multiple_orders() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        let orders = std::vec![
            NewOrder {
                quantity: 100,
                price: 50,
                fee_amount: 1,
                fee_token_asset: Address::generate(&env),
            },
            NewOrder {
                quantity: 200,
                price: 50,
                fee_amount: 1,
                fee_token_asset: Address::generate(&env),
            },
        ];

        let account = Address::generate(&env);

        for (i, new_order) in orders.into_iter().enumerate() {
            let result = place_order(
                &env,
                &mut order_book,
                OrderType::Limit,
                OrderSide::BUY,
                new_order.clone(),
                account.clone(),
            );
            assert!(result.is_ok());

            let (placed_order, matched_orders) = result.unwrap();
            assert!(matched_orders.is_empty());

            let (order_id, res_new_order) = placed_order.unwrap();

            assert_eq!(new_order, res_new_order);
            assert_eq!(
                order_id,
                OrderBookId::BuyId(PriceLevelId {
                    id: (i + 1) as u64,
                    price: new_order.price
                })
            );
        }
    }

    #[test]
    fn test_market_order_with_liquidity() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        // Add liquidity to the order book
        let existing_orders = std::vec![
            NewOrder {
                quantity: 50,
                price: 100,
                fee_amount: 1,
                fee_token_asset: Address::generate(&env),
            },
            NewOrder {
                quantity: 30,
                price: 105,
                fee_amount: 1,
                fee_token_asset: Address::generate(&env),
            },
        ];
        let account = Address::generate(&env);

        for order in existing_orders.clone() {
            let _ = place_order(
                &env,
                &mut order_book,
                OrderType::Limit,
                OrderSide::SELL,
                order,
                account.clone(),
            );
        }

        // Place a market buy order
        let market_order = NewOrder {
            quantity: 60,
            price: 0, // Price is irrelevant for market orders
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };

        let result = place_order(
            &env,
            &mut order_book,
            OrderType::Market,
            OrderSide::BUY,
            market_order.clone(),
            account.clone(),
        );

        assert!(result.is_ok());

        let (remaining_order, matched_orders) = result.unwrap();

        assert!(remaining_order.is_none());
        assert_eq!(matched_orders.len(), 2);
        assert_eq!(matched_orders.get(0).unwrap().quantity, 50);
        assert_eq!(matched_orders.get(1).unwrap().quantity, 10);
    }

    #[test]
    fn test_limit_order_matching() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        // Add liquidity to the order book
        let existing_order = NewOrder {
            quantity: 100,
            price: 100,
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };
        let account = Address::generate(&env);

        let _ = place_order(
            &env,
            &mut order_book,
            OrderType::Limit,
            OrderSide::SELL,
            existing_order.clone(),
            account.clone(),
        );

        let limit_order = NewOrder {
            quantity: 50,
            price: 105, // Higher than the existing sell price
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };

        let result = place_order(
            &env,
            &mut order_book,
            OrderType::Limit,
            OrderSide::BUY,
            limit_order.clone(),
            account.clone(),
        );

        assert!(result.is_ok());

        let (remaining_order, matched_orders) = result.unwrap();

        // Limit order should partially match the liquidity
        assert!(remaining_order.is_none());
        assert_eq!(matched_orders.len(), 1);
        assert_eq!(matched_orders.get(0).unwrap().quantity, 50);
        assert_eq!(matched_orders.get(0).unwrap().price, 100);
    }

    #[test]
    fn test_limit_order_higher_price_no_match() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        // Add liquidity to the order book
        let existing_order = NewOrder {
            quantity: 100,
            price: 100,
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };
        let account = Address::generate(&env);

        let _ = place_order(
            &env,
            &mut order_book,
            OrderType::Limit,
            OrderSide::SELL,
            existing_order.clone(),
            account.clone(),
        );

        // Place a limit buy order with a price less than the smallest sell price
        let limit_order = NewOrder {
            quantity: 50,
            price: 95, // Lower than the existing sell price
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };

        let result = place_order(
            &env,
            &mut order_book,
            OrderType::Limit,
            OrderSide::BUY,
            limit_order.clone(),
            account.clone(),
        );

        assert!(result.is_ok());

        let (remaining_order, matched_orders) = result.unwrap();

        // Limit order should not match any liquidity
        assert!(matched_orders.is_empty());
        assert!(remaining_order.is_some());
        let (order_id, res_new_order) = remaining_order.unwrap();

        assert_eq!(res_new_order, limit_order);
        assert_eq!(
            order_id,
            OrderBookId::BuyId(PriceLevelId { id: 1, price: 95 })
        );
    }

    #[test]
    fn test_partial_matches() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        // Add multiple sell orders to the order book
        let existing_orders = std::vec![
            NewOrder {
                quantity: 40,
                price: 100,
                fee_amount: 1,
                fee_token_asset: Address::generate(&env),
            },
            NewOrder {
                quantity: 60,
                price: 110,
                fee_amount: 1,
                fee_token_asset: Address::generate(&env),
            },
        ];
        let account = Address::generate(&env);

        for order in existing_orders.clone() {
            let _ = place_order(
                &env,
                &mut order_book,
                OrderType::Limit,
                OrderSide::SELL,
                order,
                account.clone(),
            );
        }

        // Place a market buy order that partially matches
        let market_order = NewOrder {
            quantity: 70,
            price: 0,
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };

        let result = place_order(
            &env,
            &mut order_book,
            OrderType::Market,
            OrderSide::BUY,
            market_order.clone(),
            account.clone(),
        );

        assert!(result.is_ok());

        let (remaining_order, matched_orders) = result.unwrap();

        // Market order should match against the first two sell orders
        assert!(remaining_order.is_none());
        assert_eq!(matched_orders.len(), 2);
        assert_eq!(matched_orders.get(0).unwrap().quantity, 40);
        assert_eq!(matched_orders.get(0).unwrap().price, 100);
        assert_eq!(matched_orders.get(1).unwrap().quantity, 30); // Partially matched
        assert_eq!(matched_orders.get(1).unwrap().price, 110);
    }

    #[test]
    fn test_high_frequency_matching() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        let account = Address::generate(&env);

        let maker_orders = std::vec![
            NewOrder {
                quantity: 50,
                price: 100,
                fee_amount: 1,
                fee_token_asset: Address::generate(&env),
            },
            NewOrder {
                quantity: 70,
                price: 95,
                fee_amount: 1,
                fee_token_asset: Address::generate(&env),
            },
        ];

        for order in maker_orders {
            place_order(
                &env,
                &mut order_book,
                OrderType::Limit,
                OrderSide::SELL,
                order,
                account.clone(),
            )
            .unwrap();
        }

        let taker_order1 = NewOrder {
            quantity: 30,
            price: 0, // market
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };
        let taker_order2 = NewOrder {
            quantity: 50,
            price: 0, // market
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };

        let result = place_order(
            &env,
            &mut order_book,
            OrderType::Market,
            OrderSide::BUY,
            taker_order1,
            account.clone(),
        );

        assert!(result.is_ok());

        let result = place_order(
            &env,
            &mut order_book,
            OrderType::Market,
            OrderSide::BUY,
            taker_order2,
            account.clone(),
        );

        assert!(result.is_ok());
        // }
    }

    #[test]
    fn test_crossed_market_orders() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        let account = Address::generate(&env);

        // Add a market buy order
        let buy_order = NewOrder {
            quantity: 100,
            price: 0, // Irrelevant for market order
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };
        let sell_order = NewOrder {
            quantity: 100,
            price: 0, // Irrelevant for market order
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };

        // Simultaneously add market orders
        let buy_result = place_order(
            &env,
            &mut order_book,
            OrderType::Market,
            OrderSide::BUY,
            buy_order.clone(),
            account.clone(),
        );

        let sell_result = place_order(
            &env,
            &mut order_book,
            OrderType::Market,
            OrderSide::SELL,
            sell_order.clone(),
            account.clone(),
        );

        assert!(buy_result.is_ok());
        assert!(sell_result.is_ok());

        let (_, buy_matches) = buy_result.unwrap();
        let (_, sell_matches) = sell_result.unwrap();

        // Both orders should match each other
        assert_eq!(buy_matches.len(), 0);
        assert_eq!(sell_matches.len(), 1);
    }

    #[test]
    fn test_exact_match_price_and_quantity() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        // Add a maker sell order to the order book
        let maker_order = NewOrder {
            quantity: 100,
            price: 50,
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };
        let account = Address::generate(&env);

        let _ = place_order(
            &env,
            &mut order_book,
            OrderType::Limit,
            OrderSide::SELL,
            maker_order.clone(),
            account.clone(),
        );

        // Place a taker buy order that matches exactly in price and quantity
        let taker_order = NewOrder {
            quantity: 100,
            price: 50,
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };

        let result = place_order(
            &env,
            &mut order_book,
            OrderType::Limit,
            OrderSide::BUY,
            taker_order.clone(),
            account.clone(),
        );

        assert!(result.is_ok());

        let (remaining_order, matched_orders) = result.unwrap();

        // Verify both orders are completely filled
        assert!(remaining_order.is_none());
        assert_eq!(matched_orders.len(), 1);
        assert_eq!(matched_orders.get(0).unwrap().quantity, 100);
        assert_eq!(matched_orders.get(0).unwrap().price, 50);
    }

    #[test]
    fn test_partial_fill_with_multiple_maker_orders() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        // Add multiple maker sell orders to the order book
        let maker_orders = std::vec![
            NewOrder {
                quantity: 40,
                price: 50,
                fee_amount: 1,
                fee_token_asset: Address::generate(&env),
            },
            NewOrder {
                quantity: 60,
                price: 55,
                fee_amount: 1,
                fee_token_asset: Address::generate(&env),
            },
            NewOrder {
                quantity: 50,
                price: 60,
                fee_amount: 1,
                fee_token_asset: Address::generate(&env),
            },
        ];
        let account = Address::generate(&env);

        for order in maker_orders.clone() {
            let _ = place_order(
                &env,
                &mut order_book,
                OrderType::Limit,
                OrderSide::SELL,
                order,
                account.clone(),
            );
        }

        // Place a taker buy order with a quantity that partially matches multiple maker orders
        let taker_order = NewOrder {
            quantity: 80,
            price: 0, // Ensures matching with lower prices first
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };

        let result = place_order(
            &env,
            &mut order_book,
            OrderType::Market,
            OrderSide::BUY,
            taker_order.clone(),
            account.clone(),
        );

        assert!(result.is_ok());

        let (remaining_order, matched_orders) = result.unwrap();

        assert!(remaining_order.is_none());

        assert_eq!(matched_orders.len(), 2);
        assert_eq!(matched_orders.get(0).unwrap().quantity, 40);
        assert_eq!(matched_orders.get(0).unwrap().price, 50);
        assert_eq!(matched_orders.get(1).unwrap().quantity, 40);
        assert_eq!(matched_orders.get(1).unwrap().price, 55);

        let remaining_maker_order =
            order_book.try_get(OrderBookId::SellId(PriceLevelId { id: 1, price: 55 }));
        assert!(remaining_maker_order.is_ok());
        assert_eq!(remaining_maker_order.unwrap().quantity, 20);

        let remaining_maker_order =
            order_book.try_get(OrderBookId::SellId(PriceLevelId { id: 1, price: 60 }));
        assert!(remaining_maker_order.is_ok());
        assert_eq!(remaining_maker_order.unwrap().quantity, 50);
    }

    #[test]
    fn test_partial_fill_with_multiple_maker_orders_of_same_price() {
        let env: Env = soroban_sdk::Env::default();
        let mut order_book = OrderBook::new(&env);

        // Add multiple maker sell orders to the order book
        let maker_orders = std::vec![
            NewOrder {
                quantity: 40,
                price: 55,
                fee_amount: 1,
                fee_token_asset: Address::generate(&env),
            },
            NewOrder {
                quantity: 60,
                price: 55,
                fee_amount: 1,
                fee_token_asset: Address::generate(&env),
            },
            NewOrder {
                quantity: 50,
                price: 55,
                fee_amount: 1,
                fee_token_asset: Address::generate(&env),
            },
        ];
        let account = Address::generate(&env);

        for order in maker_orders.clone() {
            let _ = place_order(
                &env,
                &mut order_book,
                OrderType::Limit,
                OrderSide::SELL,
                order,
                account.clone(),
            );
        }

        // Place a taker buy order with a quantity that partially matches multiple maker orders
        let taker_order = NewOrder {
            quantity: 80,
            price: 60, // Ensures matching with lower prices first
            fee_amount: 1,
            fee_token_asset: Address::generate(&env),
        };

        let result = place_order(
            &env,
            &mut order_book,
            OrderType::Market,
            OrderSide::BUY,
            taker_order.clone(),
            account.clone(),
        );

        assert!(result.is_ok());

        let (remaining_order, matched_orders) = result.unwrap();

        assert!(remaining_order.is_none());

        assert_eq!(matched_orders.len(), 2);
        assert_eq!(matched_orders.get(0).unwrap().quantity, 40);
        assert_eq!(matched_orders.get(0).unwrap().price, 55);
        assert_eq!(matched_orders.get(1).unwrap().quantity, 40);
        assert_eq!(matched_orders.get(1).unwrap().price, 55);

        let remaining_maker_order =
            order_book.try_get(OrderBookId::SellId(PriceLevelId { id: 2, price: 55 }));
        assert!(remaining_maker_order.is_ok());
        assert_eq!(remaining_maker_order.unwrap().quantity, 20);

        let remaining_maker_order =
            order_book.try_get(OrderBookId::SellId(PriceLevelId { id: 3, price: 55 }));
        assert!(remaining_maker_order.is_ok());
        assert_eq!(remaining_maker_order.unwrap().quantity, 50);
    }
}
