#![no_std]
use crate::error::Error;
use order::{NewOrder, Order, OrderSide, OrderType};
use order_executor::OrderExecutor;
use orderbook::{OrderBook, OrderBookId};
use soroban_sdk::{
    assert_with_error, contract, contractimpl, contracttype, token, Address, BytesN, Env, Vec,
};
use user_balance_manager::{UserBalanceManager, UserBalances};

mod error;
mod order;
mod order_executor;
mod orderbook;
mod payoff_sides;
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

        let base_token_decimals = token::Client::new(&env, &trading_pair.0).decimals();

        let mut order_executor = OrderExecutor::new(&env, trading_pair, base_token_decimals, side)?;

        let res = order_executor.create_order(order, user, order_type)?;

        order_executor.save_state();

        Ok(res)
    }

    pub fn cancel_order(
        env: Env,
        trading_pair: (Address, Address),
        order_id: OrderBookId,
        user: Address,
    ) -> Result<Order, Error> {
        user.require_auth();

        let base_token_decimals = token::Client::new(&env, &trading_pair.0).decimals();

        let side = match order_id {
            OrderBookId::BuyId(..) => OrderSide::BUY,
            OrderBookId::SellId(..) => OrderSide::SELL,
        };

        let mut order_executor = OrderExecutor::new(&env, trading_pair, base_token_decimals, side)?;

        let res = order_executor.cancel_order(user, order_id)?;

        order_executor.save_state();

        Ok(res)
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
