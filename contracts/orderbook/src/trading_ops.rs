use soroban_sdk::{contracttype, Env, Vec};

use crate::{
    error::Error,
    order::{Order, OrderSide, OrderType},
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
    pub taker_order: Order,
    pub maker_fills: Vec<MakerFill>,
    pub taker_fill_status: FillStatus,
}

impl PendingFill {
    pub fn new(
        taker_order: Order,
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
    taker: Order,
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
            taker_rem_q = taker_rem_q - fill_amount;
        }
    }

    if taker_rem_q == taker.quantity {
        taker_fill_status = FillStatus::None;
    }

    let pending_fill = PendingFill::new(taker, maker_fills, taker_fill_status);

    Ok(pending_fill)
}

fn finalize_matching(
    order_book: &mut OrderBook,
    pending_fill: PendingFill,
) -> Result<(FillStatus, Option<Order>), Error> {
    let mut taker_order_remaining_quantity = pending_fill.taker_order.quantity;

    for fill in pending_fill.maker_fills.iter() {
        if order_book.try_get(fill.oix.clone()).is_none() {
            return Err(Error::InvalidOrderIndex);
        }
    }

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
                let maker_order = order_book
                    .remove_order(oix)
                    .ok_or(Error::InvalidOrderIndex)?; // this should never fail because we already checked that the order exists.
                assert_eq!(maker_order, order);

                taker_order_remaining_quantity -= maker_order.quantity; // if this also filled the taker order, then we wont loop again.
            }
            // partial fill for a maker order also means a complete fill for the taker order.
            FillStatus::Partial => {
                let mut maker_order = order_book
                    .try_get(oix.clone())
                    .ok_or(Error::InvalidOrderIndex)?; // this should never fail because we already checked that the order exists.
                assert_eq!(maker_order, order);
                assert!(taker_order_remaining_quantity < maker_order.quantity);
                maker_order.quantity -= taker_order_remaining_quantity;
                order_book
                    .update_order(oix, order)
                    .ok_or(Error::InvalidOrderIndex)?;
                taker_order_remaining_quantity = 0;
            }
            FillStatus::None => unreachable!(),
        }
    }

    match pending_fill.taker_fill_status {
        FillStatus::Complete => assert_eq!(taker_order_remaining_quantity, 0),
        FillStatus::Partial => {
            assert!(pending_fill.taker_order.quantity > taker_order_remaining_quantity)
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

    Ok((pending_fill.taker_fill_status, taker_order))
}

pub fn place_order(
    env: &Env,
    order_book: &mut OrderBook,
    order_type: OrderType,
    side: OrderSide,
    order: Order,
) -> Result<Option<OrderBookId>, Error> {
    let pending_fill = fill_order(env, order_book, order, side, order_type)?;

    let (_, taker_order) = finalize_matching(order_book, pending_fill)?;

    if let Some(order) = taker_order {
        // order was not completely filled, add it to the orderbook.
        // Handle this as a GoodTillCancel time force
        return Ok(Some(match side {
            OrderSide::BUY => order_book.add_buy_order(order, env),
            OrderSide::SELL => order_book.add_sell_order(order, env),
        }));
    } else {
        Ok(None)
    }
}
