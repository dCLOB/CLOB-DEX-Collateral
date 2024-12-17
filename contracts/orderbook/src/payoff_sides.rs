use soroban_sdk::{Address, Env, Vec};

use crate::{
    error::Error,
    order::{NewOrder, Order, OrderSide},
    user_balance_manager::UserBalanceManager,
};

pub(crate) struct PayOffWithSides<'a> {
    env: &'a Env,
    token_to_withdraw: Address,
    token_to_receive: Address,
    withdraw_calculation_func: fn(u128, u128, u32) -> i128,
    receive_calculation_func: fn(u128, u128, u32) -> i128,
    base_token_decimals: u32,
}

fn multiply_price_and_quantity(price: u128, quantity: u128, decimals: u32) -> i128 {
    (price * quantity) as i128 / 10_i128.pow(decimals)
}

fn return_quantity(_price: u128, quantity: u128, _decimals: u32) -> i128 {
    quantity as i128
}

impl<'a> PayOffWithSides<'a> {
    pub fn new(
        env: &'a Env,
        side: OrderSide,
        trading_pair: &(Address, Address),
        base_token_decimals: u32,
    ) -> Self {
        match side {
            OrderSide::BUY => Self {
                env,
                token_to_withdraw: trading_pair.1.clone(),
                token_to_receive: trading_pair.0.clone(),
                withdraw_calculation_func: multiply_price_and_quantity,
                receive_calculation_func: return_quantity,
                base_token_decimals,
            },
            OrderSide::SELL => Self {
                env,
                token_to_withdraw: trading_pair.0.clone(),
                token_to_receive: trading_pair.1.clone(),
                withdraw_calculation_func: return_quantity,
                receive_calculation_func: multiply_price_and_quantity,
                base_token_decimals,
            },
        }
    }

    fn taker_withdraw_token(&self) -> Address {
        self.token_to_withdraw.clone()
    }

    fn taker_receive_token(&self) -> Address {
        self.token_to_receive.clone()
    }

    fn maker_withdraw_token(&self) -> Address {
        self.token_to_receive.clone()
    }

    fn maker_receive_token(&self) -> Address {
        self.token_to_withdraw.clone()
    }

    pub fn taker_withdraw_amount(&self, price: u128, quantity: u128) -> i128 {
        (self.withdraw_calculation_func)(price, quantity, self.base_token_decimals)
    }

    pub fn maker_withdraw_amount(&self, price: u128, quantity: u128) -> i128 {
        (self.receive_calculation_func)(price, quantity, self.base_token_decimals)
    }

    pub fn taker_receive_amount(&self, price: u128, quantity: u128) -> i128 {
        (self.receive_calculation_func)(price, quantity, self.base_token_decimals)
    }

    pub fn maker_receive_amount(&self, price: u128, quantity: u128) -> i128 {
        (self.withdraw_calculation_func)(price, quantity, self.base_token_decimals)
    }

    pub fn pay_off_with_makers(&self, maker_orders: &Vec<Order>) -> Result<(), Error> {
        for order in maker_orders {
            let withdraw_balance_manager =
                UserBalanceManager::new(order.account.clone(), self.maker_withdraw_token());

            let mut withdraw_balances = withdraw_balance_manager.read_user_balance(self.env);
            let withdraw_amount = self.maker_withdraw_amount(order.price, order.quantity);

            withdraw_balances.balance_in_trading -= withdraw_amount;
            withdraw_balance_manager.write_user_balance(self.env, &withdraw_balances);

            let receiving_balance_manager =
                UserBalanceManager::new(order.account.clone(), self.maker_receive_token());
            let mut receiving_balances = receiving_balance_manager.read_user_balance(self.env);

            let receive_amount = self.maker_receive_amount(order.price, order.quantity);

            receiving_balances.balance += receive_amount;
            receiving_balance_manager.write_user_balance(self.env, &receiving_balances);
        }

        Ok(())
    }

    pub fn pay_of_with_taker(
        &self,
        order: &NewOrder,
        user: Address,
        filled_quantity: u128,
        withdraw_amount: i128,
        receive_amount: i128,
    ) {
        let withdraw_balance_manager =
            UserBalanceManager::new(user.clone(), self.taker_withdraw_token());
        let mut withdraw_balances = withdraw_balance_manager.read_user_balance(&self.env);

        withdraw_balances.balance -= withdraw_amount;

        let receiving_balance_manager =
            UserBalanceManager::new(user.clone(), self.taker_receive_token());
        let mut receiving_balances = receiving_balance_manager.read_user_balance(&self.env);

        receiving_balances.balance += receive_amount;
        receiving_balance_manager.write_user_balance(self.env, &receiving_balances);
        receiving_balance_manager.emit_balance_update(self.env, receiving_balances);

        if filled_quantity < order.quantity {
            let not_filled_withdraw_amount =
                self.taker_withdraw_amount(order.price, order.quantity - filled_quantity);
            withdraw_balances.balance -= not_filled_withdraw_amount;
            withdraw_balances.balance_in_trading += not_filled_withdraw_amount;
        }

        withdraw_balance_manager.write_user_balance(self.env, &withdraw_balances);
        withdraw_balance_manager.emit_balance_update(self.env, withdraw_balances);
    }

    pub fn pay_off_for_cancellation(&self, user: Address, order: &Order) {
        let user_balance_manager = UserBalanceManager::new(user, self.token_to_withdraw.clone());
        let mut balances = user_balance_manager.read_user_balance(&self.env);

        let token_trading_amount =
            (self.withdraw_calculation_func)(order.price, order.quantity, self.base_token_decimals);

        balances.balance += token_trading_amount;
        balances.balance_in_trading -= token_trading_amount;

        user_balance_manager.write_user_balance(self.env, &balances);
        user_balance_manager.emit_balance_update(self.env, balances);
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use soroban_sdk::{testutils::Address as TestAddress, vec, Address, Env};

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

    #[test]
    fn test_payoff_initialization_buy_side() {
        let env = Env::default();
        let token_base = Address::generate(&env);
        let token_quote = Address::generate(&env);
        let trading_pair = (token_base.clone(), token_quote.clone());

        let payoff = PayOffWithSides::new(&env, OrderSide::BUY, &trading_pair, 18);

        assert_eq!(payoff.taker_withdraw_token(), token_quote);
        assert_eq!(payoff.taker_receive_token(), token_base);
        assert_eq!(payoff.maker_withdraw_token(), token_base);
        assert_eq!(payoff.maker_receive_token(), token_quote);
    }

    #[test]
    fn test_payoff_initialization_sell_side() {
        let env = Env::default();
        let token_base = Address::generate(&env);
        let token_quote = Address::generate(&env);
        let trading_pair = (token_base.clone(), token_quote.clone());

        let payoff = PayOffWithSides::new(&env, OrderSide::SELL, &trading_pair, 18);

        assert_eq!(payoff.taker_withdraw_token(), token_base);
        assert_eq!(payoff.taker_receive_token(), token_quote);
        assert_eq!(payoff.maker_withdraw_token(), token_quote);
        assert_eq!(payoff.maker_receive_token(), token_base);
    }

    #[test]
    fn test_multiply_price_and_quantity() {
        let price = 100_u128; // price = 100
        let quantity = 5_u128; // quantity = 5
        let decimals = 2; // decimals = 2
        let result = multiply_price_and_quantity(price, quantity, decimals);
        assert_eq!(result, 5);

        let price = 100_u128; // price = 100
        let quantity = 5_u128; // quantity = 5
        let decimals = 0; // decimals = 0
        let result = multiply_price_and_quantity(price, quantity, decimals);
        assert_eq!(result, 500);
    }

    #[test]
    fn test_return_quantity() {
        let price = 0_u128;
        let quantity = 10_u128;
        let decimals = 0;
        let result = return_quantity(price, quantity, decimals);
        assert_eq!(result, 10);
    }

    #[test]
    fn test_pay_off_with_makers() {
        let env = Env::default();

        contract_test_scope!(env, {
            let token_base = Address::generate(&env);
            let token_quote = Address::generate(&env);

            let payoff = PayOffWithSides::new(
                &env,
                OrderSide::BUY,
                &(token_base.clone(), token_quote.clone()),
                18,
            );

            let maker_orders = vec![
                &env,
                Order {
                    order_id: 1,
                    account: Address::generate(&env),
                    price: 100,
                    quantity: 5,
                    fee_amount: 1,
                    fee_token_asset: token_quote.clone(),
                },
                Order {
                    order_id: 2,
                    account: Address::generate(&env),
                    price: 150,
                    quantity: 10,
                    fee_amount: 1,
                    fee_token_asset: token_quote.clone(),
                },
            ];

            for order in &maker_orders {
                mock_user_balance_manager(
                    &env,
                    order.account.clone(),
                    token_base.clone(),
                    1000,
                    order.quantity as i128,
                );
                mock_user_balance_manager(&env, order.account.clone(), token_quote.clone(), 500, 0);
            }

            let result = payoff.pay_off_with_makers(&maker_orders);
            assert!(result.is_ok());

            for order in maker_orders {
                let withdraw_manager =
                    UserBalanceManager::new(order.account.clone(), token_base.clone());
                let withdraw_balance = withdraw_manager.read_user_balance(&env);

                let receive_manager =
                    UserBalanceManager::new(order.account.clone(), token_quote.clone());
                let receive_balance = receive_manager.read_user_balance(&env);

                let receive_amount = payoff.maker_receive_amount(order.price, order.quantity);

                assert_eq!(withdraw_balance.balance_in_trading, 0);
                assert_eq!(withdraw_balance.balance, 1000);
                assert_eq!(receive_balance.balance - receive_amount, 500);
                assert_eq!(receive_balance.balance_in_trading, 0);
            }
        });
    }

    #[test]
    fn test_pay_off_with_taker() {
        let env = Env::default();
        contract_test_scope!(env, {
            let token_base = Address::generate(&env);
            let token_quote = Address::generate(&env);

            let payoff = PayOffWithSides::new(
                &env,
                OrderSide::BUY,
                &(token_base.clone(), token_quote.clone()),
                0, // in order to simplify calculations set it to zero
            );

            let taker_account = Address::generate(&env);
            let order = NewOrder {
                quantity: 10,
                price: 150,
                fee_amount: 1,
                fee_token_asset: token_quote.clone(),
            };

            mock_user_balance_manager(&env, taker_account.clone(), token_base.clone(), 500, 0);
            mock_user_balance_manager(&env, taker_account.clone(), token_quote.clone(), 1500, 0);

            payoff.pay_of_with_taker(&order, taker_account.clone(), 6, 6 * 150, 5);

            let withdraw_manager =
                UserBalanceManager::new(taker_account.clone(), token_quote.clone());
            let withdraw_balance = withdraw_manager.read_user_balance(&env);

            let receive_manager =
                UserBalanceManager::new(taker_account.clone(), token_base.clone());
            let receive_balance = receive_manager.read_user_balance(&env);

            assert_eq!(withdraw_balance.balance, 0);
            assert_eq!(withdraw_balance.balance_in_trading, 4 * 150);
            assert_eq!(receive_balance.balance, 500 + 5);
            assert_eq!(receive_balance.balance_in_trading, 0);
        });
    }

    #[test]
    fn test_pay_off_for_cancellation() {
        let env = Env::default();

        contract_test_scope!(env, {
            let token_base = Address::generate(&env);
            let token_quote = Address::generate(&env);

            let payoff = PayOffWithSides::new(
                &env,
                OrderSide::SELL,
                &(token_base.clone(), token_quote.clone()),
                18,
            );

            let user = Address::generate(&env);
            let order = Order {
                order_id: 1,
                account: user.clone(),
                price: 100,
                quantity: 5,
                fee_amount: 1,
                fee_token_asset: token_quote.clone(),
            };

            let balance_manager =
                mock_user_balance_manager(&env, user.clone(), token_base.clone(), 1000, 500);

            payoff.pay_off_for_cancellation(user.clone(), &order);

            let balances = balance_manager.read_user_balance(&env);
            let token_trading_amount = payoff.taker_withdraw_amount(order.price, order.quantity);

            assert_eq!(balances.balance, 1000 + token_trading_amount);
            assert_eq!(balances.balance_in_trading, 500 - token_trading_amount);
        });
    }
}
