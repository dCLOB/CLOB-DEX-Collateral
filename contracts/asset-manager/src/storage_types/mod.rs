pub(crate) mod token_whitelisted;
pub(crate) mod user_balance;

use soroban_sdk::{contracttype, Address};

// pub(crate) const SHARED_BUMP_AMOUNT: u32 = 69120; // 4 days
pub(crate) const BALANCE_BUMP_AMOUNT: u32 = 518400; // 30 days

// Data Keys for Pool' Storage Data
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Owner,           // Address of the account Owner
    OperatorManager, // Address of the Operator Manager
}

#[derive(Clone)]
#[contracttype]
pub struct UserBalance {
    pub user: Address,
    pub token: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct TokenWhitelisted {
    pub token: Address,
}
