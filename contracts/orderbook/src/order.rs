use soroban_sdk::{contracttype, Address};

use crate::order_statistic_tree::node::Key;

#[contracttype]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    BUY,
    SELL,
}

#[contracttype]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderType {
    Limit,
    Market,
}

#[contracttype]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    pub order_id: Key,
    pub account: Address,
    pub quantity: u128,
    pub price: u128,
    pub fee_amount: u128,
    pub fee_token_asset: Address,
    pub timestamp: u64,
}
