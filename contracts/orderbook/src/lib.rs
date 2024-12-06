#![no_std]
use crate::error::Error;
use order::{NewOrder, Order, OrderSide, OrderType};
use orderbook::{OrderBook, OrderBookId};
use soroban_sdk::{
    assert_with_error, contract, contractimpl, contracttype, token, Address, BytesN, Env, Vec,
};
use trading_ops::place_order;
use user_balance_manager::{UserBalanceManager, UserBalances};

mod error;
mod order;
mod orderbook;
mod price_level_store;
mod price_store;
mod trading_ops;
mod user_balance_manager;

pub(crate) const USER_DATA_BUMP_AMOUNT: u32 = 518400; // 30 days
pub(crate) const PERSISTENT_THRESHOLD: u32 = 86400; // 1 day

#[contract]
struct Contract {}

#[contracttype]
struct OrderBookStoreId {
    token1: Address,
    token2: Address,
}

fn pay_off_with_makers(
    env: &Env,
    pay_off_with_sides: PayOffWithSides,
    maker_orders: &Vec<Order>,
) -> Result<(), Error> {
    for order in maker_orders {
        let withdraw_balance_manager = UserBalanceManager::new(
            order.account.clone(),
            pay_off_with_sides.maker_withdraw_token(),
        );

        let mut withdraw_balances = withdraw_balance_manager.read_user_balance(env);
        let withdraw_amount = pay_off_with_sides.maker_withdraw_amount(order.price, order.quantity);

        withdraw_balances.balance_in_trading -= withdraw_amount;
        withdraw_balance_manager.write_user_balance(env, &withdraw_balances);

        let receiving_balance_manager = UserBalanceManager::new(
            order.account.clone(),
            pay_off_with_sides.maker_receive_token(),
        );
        let mut receiving_balances = receiving_balance_manager.read_user_balance(&env);

        let receive_amount = pay_off_with_sides.maker_receive_amount(order.price, order.quantity);

        receiving_balances.balance += receive_amount;
        receiving_balance_manager.write_user_balance(&env, &receiving_balances);
    }

    Ok(())
}

fn multiply_price_and_quantity(price: u128, quantity: u128, decimals: u32) -> i128 {
    (price * quantity) as i128 / 10_i128.pow(decimals)
}

fn return_quantity(_price: u128, quantity: u128, _decimals: u32) -> i128 {
    quantity as i128
}

struct PayOffWithSides {
    token_to_withdraw: Address,
    token_to_receive: Address,
    withdraw_calculation_func: fn(u128, u128, u32) -> i128,
    receive_calculation_func: fn(u128, u128, u32) -> i128,
    base_token_decimals: u32,
}

impl PayOffWithSides {
    pub fn create(
        side: OrderSide,
        trading_pair: &(Address, Address),
        base_token_decimals: u32,
    ) -> Self {
        match side {
            OrderSide::BUY => Self {
                token_to_withdraw: trading_pair.1.clone(),
                token_to_receive: trading_pair.0.clone(),
                withdraw_calculation_func: multiply_price_and_quantity,
                receive_calculation_func: return_quantity,
                base_token_decimals,
            },
            OrderSide::SELL => Self {
                token_to_withdraw: trading_pair.0.clone(),
                token_to_receive: trading_pair.1.clone(),
                withdraw_calculation_func: return_quantity,
                receive_calculation_func: multiply_price_and_quantity,
                base_token_decimals,
            },
        }
    }

    pub fn taker_withdraw_token(&self) -> Address {
        self.token_to_withdraw.clone()
    }

    pub fn taker_receive_token(&self) -> Address {
        self.token_to_receive.clone()
    }

    pub fn maker_withdraw_token(&self) -> Address {
        self.token_to_receive.clone()
    }

    pub fn maker_receive_token(&self) -> Address {
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
}

fn create_order(
    env: &Env,
    order_book: &mut OrderBook,
    pay_off_with_sides: PayOffWithSides,
    order: NewOrder,
    user: Address,
    order_type: OrderType,
    side: OrderSide,
) -> Result<(OrderBookId, Vec<Order>), Error> {
    let (taker_order, maker_orders) = place_order(
        &env,
        order_book,
        order_type,
        side,
        order.clone(),
        user.clone(),
    )?;

    let mut total_filled_quantity = 0;
    let mut total_withdraw_amount = 0;
    let mut total_receive_amount = 0;

    for order in maker_orders.iter() {
        total_filled_quantity += order.quantity;
        total_withdraw_amount +=
            pay_off_with_sides.taker_withdraw_amount(order.price, order.quantity);
        total_receive_amount +=
            pay_off_with_sides.taker_receive_amount(order.price, order.quantity);
    }

    let withdraw_balance_manager =
        UserBalanceManager::new(user.clone(), pay_off_with_sides.taker_withdraw_token());
    let mut withdraw_balances = withdraw_balance_manager.read_user_balance(&env);

    withdraw_balances.balance -= total_withdraw_amount;

    let receiving_balance_manager =
        UserBalanceManager::new(user.clone(), pay_off_with_sides.taker_receive_token());
    let mut receiving_balances = receiving_balance_manager.read_user_balance(&env);

    receiving_balances.balance += total_receive_amount;
    receiving_balance_manager.write_user_balance(env, &receiving_balances);
    receiving_balance_manager.emit_balance_update(env, receiving_balances);

    if total_filled_quantity < order.quantity {
        let not_filled_withdraw_amount = pay_off_with_sides
            .taker_withdraw_amount(order.price, order.quantity - total_filled_quantity);
        withdraw_balances.balance -= not_filled_withdraw_amount;
        withdraw_balances.balance_in_trading += not_filled_withdraw_amount;
    }

    withdraw_balance_manager.write_user_balance(env, &withdraw_balances);
    withdraw_balance_manager.emit_balance_update(env, withdraw_balances);

    pay_off_with_makers(&env, pay_off_with_sides, &maker_orders)?;

    Ok((
        taker_order
            .map(|el| el.0)
            .unwrap_or(OrderBookId::buy_id(0, 0)),
        maker_orders,
    ))
}

#[contractimpl]
impl Contract {
    pub fn initialize(env: Env, token_pairs: Vec<(Address, Address)>) {
        // let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        // admin.require_auth();

        for (token1, token2) in token_pairs {
            env.storage()
                .persistent()
                .set(&OrderBookStoreId { token1, token2 }, &OrderBook::new(&env));
        }
    }

    pub fn create_order(
        env: Env,
        trading_pair: (Address, Address),
        order_type: OrderType,
        side: OrderSide,
        order: NewOrder,
        user: Address,
    ) -> Result<(OrderBookId, Vec<Order>), Error> {
        user.require_auth();

        let mut order_book: OrderBook = env
            .storage()
            .persistent()
            .get(&OrderBookStoreId {
                token1: trading_pair.0.clone(),
                token2: trading_pair.1.clone(),
            })
            .unwrap();

        let base_token_decimals = token::Client::new(&env, &trading_pair.0).decimals();

        let pay_off_with_sides = PayOffWithSides::create(side, &trading_pair, base_token_decimals);

        let res = create_order(
            &env,
            &mut order_book,
            pay_off_with_sides,
            order,
            user,
            order_type,
            side,
        );

        env.storage().persistent().set(
            &OrderBookStoreId {
                token1: trading_pair.0.clone(),
                token2: trading_pair.1.clone(),
            },
            &order_book,
        );

        res
    }

    pub fn cancel_order(
        env: Env,
        trading_pair: (Address, Address),
        order_id: OrderBookId,
        user: Address,
    ) -> Result<Order, Error> {
        user.require_auth();

        let mut order_book: OrderBook = env
            .storage()
            .persistent()
            .get(&OrderBookStoreId {
                token1: trading_pair.0.clone(),
                token2: trading_pair.1.clone(),
            })
            .unwrap();

        let res = order_book.remove_order(order_id.clone());

        if let Ok(order) = &res {
            let (token_to_return, return_calculation_func) = match order_id {
                OrderBookId::BuyId(..) => (
                    trading_pair.1.clone(),
                    multiply_price_and_quantity as fn(u128, u128, u32) -> i128,
                ),
                OrderBookId::SellId(..) => (
                    trading_pair.0.clone(),
                    return_quantity as fn(u128, u128, u32) -> i128,
                ),
            };

            let user_balance_manager = UserBalanceManager::new(user, token_to_return);
            let mut balances = user_balance_manager.read_user_balance(&env);

            let token_decimals_to_receive = token::Client::new(&env, &trading_pair.0).decimals();

            let token_trading_amount =
                return_calculation_func(order.price, order.quantity, token_decimals_to_receive);

            balances.balance += token_trading_amount;
            balances.balance_in_trading -= token_trading_amount;

            user_balance_manager.write_user_balance(&env, &balances);
            user_balance_manager.emit_balance_update(&env, balances);
        }

        env.storage().persistent().set(
            &OrderBookStoreId {
                token1: trading_pair.0.clone(),
                token2: trading_pair.1.clone(),
            },
            &order_book,
        );

        res
    }

    pub fn deposit(e: Env, user: Address, token: Address, amount: i128) {
        user.require_auth();
        assert_with_error!(&e, amount > 0, Error::AmountMustBePositive);

        let client = token::Client::new(&e, &token);
        client.transfer(&user, &e.current_contract_address(), &amount);

        let user_balance_manager = UserBalanceManager::new(user.clone(), token.clone());
        let mut balances = user_balance_manager.read_user_balance(&e);
        balances.balance += amount;
        user_balance_manager.write_user_balance(&e, &balances);

        user_balance_manager.emit_balance_update(&e, balances);
        user_balance_manager.emit_deposit(&e, amount);
    }

    pub fn withdraw(e: Env, user: Address, token: Address, amount: i128) {
        user.require_auth();
        assert_with_error!(&e, amount > 0, Error::AmountMustBePositive);

        let user_balance_manager = UserBalanceManager::new(user.clone(), token.clone());
        let mut balances = user_balance_manager.read_user_balance(&e);

        assert_with_error!(e, balances.balance >= amount, Error::BalanceNotEnough);
        balances.balance -= amount;
        user_balance_manager.write_user_balance(&e, &balances);

        user_balance_manager.emit_balance_update(&e, balances);

        let client = token::Client::new(&e, &token);
        client.transfer(&e.current_contract_address(), &user, &amount);

        user_balance_manager.emit_withdraw(&e, amount);
    }

    pub fn balances(e: Env, user: Address, token: Address) -> UserBalances {
        let user_balance_manager = UserBalanceManager::new(user.clone(), token.clone());
        user_balance_manager.read_user_balance(&e)
    }

    pub fn upgrade(e: Env, new_wasm_hash: BytesN<32>) {
        // let admin: Address = e.storage().instance().get(&DataKey::Admin).unwrap();
        // admin.require_auth();

        e.deployer().update_current_contract_wasm(new_wasm_hash);
    }
}
