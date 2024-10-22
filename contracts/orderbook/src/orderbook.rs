use soroban_sdk::Address;

use crate::{error::Error, order::OrderSide, order_statistic_tree::node::Key};

pub fn place_order(
    order_side: OrderSide,
    user: Address,
    price: u128,
    margin: u128,
    size: u128,
) -> Result<(), Error> {
    todo!()
}

pub fn cancel_order(order_id: Key, user: Address) -> Result<(), Error> {
    todo!()
}

pub fn list_user_orders(user: Address, offset: usize, limit: usize) -> Vec<Key> {
    todo!()
}

pub fn market_price(order_side: OrderSide) -> u128 {
    todo!()
}

fn is_market_order(order_side: OrderSide, price: u128) -> bool {
    todo!()
}
