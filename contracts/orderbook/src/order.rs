use soroban_sdk::{contracttype, Address};

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
    pub order_id: u64,
    pub account: Address,
    pub quantity: u128,
    pub price: u128,
    pub fee_amount: u128,
    pub fee_token_asset: Address,
}

#[contracttype]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewOrder {
    pub quantity: u128,
    pub price: u128,
    pub fee_amount: u128,
    pub fee_token_asset: Address,
}

#[contracttype]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewAccountOrder {
    pub quantity: u128,
    pub price: u128,
    pub fee_amount: u128,
    pub fee_token_asset: Address,
    pub account: Address,
}

pub trait AddField<F, T> {
    fn into_order(self, field: F) -> T;
}

impl AddField<Address, NewAccountOrder> for NewOrder {
    fn into_order(self, field: Address) -> NewAccountOrder {
        NewAccountOrder {
            quantity: self.quantity,
            price: self.price,
            fee_amount: self.fee_amount,
            fee_token_asset: self.fee_token_asset,
            account: field,
        }
    }
}

impl AddField<u64, Order> for NewAccountOrder {
    fn into_order(self, field: u64) -> Order {
        Order {
            order_id: field,
            account: self.account,
            quantity: self.quantity,
            price: self.price,
            fee_amount: self.fee_amount,
            fee_token_asset: self.fee_token_asset,
        }
    }
}
