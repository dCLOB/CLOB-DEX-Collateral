use soroban_sdk::{Address, Env, Vec};

use crate::{
    error::Error,
    order::{NewOrder, Order, OrderSide, OrderType},
    orderbook::{OrderBook, OrderBookId},
    payoff_sides::PayOffWithSides,
    trading_ops::place_order,
    OrderBookStoreId,
};

pub struct OrderExecutor<'a> {
    pay_off: PayOffWithSides<'a>,
    order_book: OrderBook,
    trading_pair: (Address, Address),
    side: OrderSide,
    env: &'a Env,
}

impl<'a> OrderExecutor<'a> {
    pub fn new(
        env: &'a Env,
        trading_pair: (Address, Address),
        base_token_decimals: u32,
        side: OrderSide,
    ) -> Result<Self, Error> {
        let order_book: OrderBook = env
            .storage()
            .persistent()
            .get(&OrderBookStoreId {
                token1: trading_pair.0.clone(),
                token2: trading_pair.1.clone(),
            })
            .ok_or(Error::OrderBookNotFound)?;

        let pay_off_with_sides =
            PayOffWithSides::new(env, side, &trading_pair, base_token_decimals);

        Ok(Self {
            pay_off: pay_off_with_sides,
            trading_pair,
            order_book,
            side,
            env,
        })
    }

    pub fn create_order(
        &mut self,
        order: NewOrder,
        user: Address,
        order_type: OrderType,
    ) -> Result<(OrderBookId, Vec<Order>), Error> {
        let (taker_order, maker_orders) = place_order(
            &self.env,
            &mut self.order_book,
            order_type,
            self.side,
            order.clone(),
            user.clone(),
        )?;

        let mut total_filled_quantity = 0;
        let mut total_withdraw_amount = 0;
        let mut total_receive_amount = 0;

        for order in maker_orders.iter() {
            total_filled_quantity += order.quantity;
            total_withdraw_amount += self
                .pay_off
                .taker_withdraw_amount(order.price, order.quantity);
            total_receive_amount += self
                .pay_off
                .taker_receive_amount(order.price, order.quantity);
        }

        self.pay_off.pay_of_with_taker(
            &order,
            user,
            total_filled_quantity,
            total_withdraw_amount,
            total_receive_amount,
        );

        self.pay_off.pay_off_with_makers(&maker_orders)?;

        Ok((
            taker_order
                .map(|el| el.0)
                .unwrap_or(OrderBookId::buy_id(0, 0)),
            maker_orders,
        ))
    }

    pub fn cancel_order(&mut self, user: Address, order_id: OrderBookId) -> Result<Order, Error> {
        let order = self.order_book.remove_order(order_id.clone())?;

        self.pay_off.pay_off_for_cancellation(user, &order);

        Ok(order)
    }

    pub fn save_state(&self) {
        self.env.storage().persistent().set(
            &OrderBookStoreId {
                token1: self.trading_pair.0.clone(),
                token2: self.trading_pair.1.clone(),
            },
            &self.order_book,
        );
    }
}

#[cfg(test)]
mod order_executor_tests {
    use crate::{orderbook::PriceLevelId, user_balance_manager::UserBalanceManager};

    use super::*;
    use soroban_sdk::{testutils::Address as TestAddress, Address, Env};

    macro_rules! contract_test_scope {
        ($env:expr, $code:block) => {{
            let id = $env.register(crate::Contract {}, ());
            $env.as_contract(&id, || $code);
        }};
    }

    fn mock_user_balance_manager(
        env: &Env,
        user: Address,
        token: Address,
        initial_balance: i128,
        initial_balance_in_trading: i128, // New parameter
    ) -> UserBalanceManager {
        let manager = UserBalanceManager::new(user.clone(), token.clone());
        let mut balances = manager.read_user_balance(env);
        balances.balance = initial_balance;
        balances.balance_in_trading = initial_balance_in_trading; // Set specific trading balance
        manager.write_user_balance(env, &balances);
        manager
    }

    fn mock_order_book(env: &Env, trading_pair: (Address, Address)) -> OrderBook {
        let order_book = OrderBook::new(env);
        env.storage().persistent().set(
            &OrderBookStoreId {
                token1: trading_pair.0.clone(),
                token2: trading_pair.1.clone(),
            },
            &order_book,
        );
        order_book
    }

    #[test]
    fn test_order_executor_initialization() {
        let env = Env::default();

        contract_test_scope!(env, {
            let token_base = Address::generate(&env);
            let token_quote = Address::generate(&env);
            let trading_pair = (token_base.clone(), token_quote.clone());

            mock_order_book(&env, trading_pair.clone());

            let executor = OrderExecutor::new(&env, trading_pair.clone(), 18, OrderSide::BUY);

            assert!(executor.is_ok());
            let executor = executor.unwrap();

            assert_eq!(executor.trading_pair, trading_pair);
            assert_eq!(executor.side, OrderSide::BUY);
        });
    }

    #[test]
    fn test_create_order() {
        let env = Env::default();

        contract_test_scope!(env, {
            let token_base = Address::generate(&env);
            let token_quote = Address::generate(&env);
            let trading_pair = (token_base.clone(), token_quote.clone());

            mock_order_book(&env, trading_pair.clone());

            let mut executor =
                OrderExecutor::new(&env, trading_pair.clone(), 18, OrderSide::BUY).unwrap();

            let user = Address::generate(&env);
            let new_order = NewOrder {
                quantity: 10,
                price: 100,
                fee_amount: 1,
                fee_token_asset: token_quote.clone(),
            };

            mock_user_balance_manager(&env, user.clone(), token_base.clone(), 500, 0);
            mock_user_balance_manager(&env, user.clone(), token_quote.clone(), 1000, 0);

            let result = executor.create_order(new_order.clone(), user.clone(), OrderType::Limit);

            assert!(result.is_ok());
            let (order_book_id, maker_orders) = result.unwrap();

            assert!(maker_orders.is_empty());
            assert!(matches!(
                order_book_id,
                OrderBookId::BuyId(PriceLevelId { id: 1, price: 100 })
            ));
        });
    }

    #[test]
    fn test_cancel_order() {
        let env = Env::default();

        contract_test_scope!(env, {
            let token_base = Address::generate(&env);
            let token_quote = Address::generate(&env);
            let trading_pair = (token_base.clone(), token_quote.clone());

            mock_order_book(&env, trading_pair.clone());

            let mut executor =
                OrderExecutor::new(&env, trading_pair.clone(), 18, OrderSide::BUY).unwrap();

            let user = Address::generate(&env);
            let new_order = NewOrder {
                quantity: 10,
                price: 100,
                fee_amount: 1,
                fee_token_asset: token_quote.clone(),
            };

            mock_user_balance_manager(&env, user.clone(), token_base.clone(), 500, 0);
            mock_user_balance_manager(&env, user.clone(), token_quote.clone(), 1000, 0);

            let (order_book_id, _) = executor
                .create_order(new_order.clone(), user.clone(), OrderType::Limit)
                .unwrap();

            let result = executor.cancel_order(user.clone(), order_book_id);

            assert!(result.is_ok());
            let canceled_order = result.unwrap();

            assert_eq!(canceled_order.quantity, new_order.quantity);
            assert_eq!(canceled_order.price, new_order.price);
        });
    }

    #[test]
    fn test_save_state() {
        let env = Env::default();

        contract_test_scope!(env, {
            let token_base = Address::generate(&env);
            let token_quote = Address::generate(&env);
            let trading_pair = (token_base.clone(), token_quote.clone());

            mock_order_book(&env, trading_pair.clone());

            let mut executor =
                OrderExecutor::new(&env, trading_pair.clone(), 18, OrderSide::BUY).unwrap();

            let user = Address::generate(&env);
            let new_order = NewOrder {
                quantity: 10,
                price: 100,
                fee_amount: 1,
                fee_token_asset: token_quote.clone(),
            };

            mock_user_balance_manager(&env, user.clone(), token_base.clone(), 500, 0);
            mock_user_balance_manager(&env, user.clone(), token_quote.clone(), 1000, 0);

            let (order_id, _) = executor
                .create_order(new_order.clone(), user.clone(), OrderType::Limit)
                .unwrap();

            executor.save_state();

            let stored_order_book: Option<OrderBook> =
                env.storage().persistent().get(&OrderBookStoreId {
                    token1: trading_pair.0.clone(),
                    token2: trading_pair.1.clone(),
                });

            assert!(stored_order_book.is_some());
            assert!(stored_order_book.unwrap().try_get(order_id).is_ok());
        });
    }
}
