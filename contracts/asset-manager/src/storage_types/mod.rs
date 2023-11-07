pub(crate) mod pair_manager;
pub(crate) mod token_manager;
pub(crate) mod user_balance_manager;

use soroban_sdk::{contracttype, Address, String};

// pub(crate) const SHARED_BUMP_AMOUNT: u32 = 69120; // 4 days
pub(crate) const BALANCE_BUMP_AMOUNT: u32 = 518400; // 30 days

#[contracttype]
#[derive(PartialEq, Clone)]
pub enum ListingStatus {
    Listed,
    Delisted,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Owner,           // Address of the account Owner
    OperatorManager, // Address of the Operator Manager
}

#[derive(Clone)]
#[contracttype]
pub struct UserBalanceManager {
    pub user: Address,
    pub token: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct TokenManager {
    pub token: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct PairManager {
    pub symbol: String,
}
