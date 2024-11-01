#![no_std]
use order::{Order, OrderSide, OrderType};
use orderbook::{OrderBook, OrderBookId};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};
use trading_ops::place_order;

mod error;
mod node_impl;
mod order;
mod order_statistic_tree;
mod orderbook;
mod price_level_store;
mod price_store;
mod storage_tree;
#[cfg(test)]
mod test;
mod trading_ops;

#[contract]
struct Contract {}

#[contracttype]
struct OrderBookStoreId {
    token1: Address,
    token2: Address,
}

#[contractimpl]
impl Contract {
    pub fn create_new_trading_pair(env: Env, token1: Address, token2: Address) {
        env.storage()
            .persistent()
            .set(&OrderBookStoreId { token1, token2 }, &OrderBook::new(&env));
    }

    pub fn create_order(
        env: Env,
        pair: (Address, Address),
        order_type: OrderType,
        side: OrderSide,
        order: Order,
    ) -> Option<OrderBookId> {
        let mut order_book: OrderBook = env
            .storage()
            .persistent()
            .get(&OrderBookStoreId {
                token1: pair.0,
                token2: pair.1,
            })
            .unwrap();

        let order = place_order(&env, &mut order_book, order_type, side, order).unwrap();

        order
    }
}
