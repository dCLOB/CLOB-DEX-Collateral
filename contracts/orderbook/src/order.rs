use soroban_sdk::{contracttype, Address};

use crate::order_statistic_tree::node::Key;

#[contracttype]
#[derive(Debug)]
pub enum OrderSide {
    BUY,
    SELL,
}

#[contracttype]
#[derive(Debug)]
pub enum OrderType {
    /// Limit order.
    Limit,
    /// Market order.
    Market,
}

#[contracttype]
pub struct Order {
    pub order_id: Key,
    pub account: Address,
    pub quantity: u128,
    pub price: u128,
    pub fee_amount: u128,
    pub fee_token_asset: Address,
    pub timestamp: u64,
}
