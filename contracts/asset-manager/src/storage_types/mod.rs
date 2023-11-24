pub(crate) mod pair_manager;
pub(crate) mod public_key_manager;
pub(crate) mod token_manager;
pub(crate) mod user_balance_manager;
pub(crate) mod withdraw_request_manager;

use soroban_sdk::{contracttype, Address, String};

// pub(crate) const SHARED_BUMP_AMOUNT: u32 = 69120; // 4 days
pub(crate) const USER_DATA_BUMP_AMOUNT: u32 = 518400; // 30 days

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
    FeeCollector,    // Address of the Fee Collector
    WithdrawId,      // u64 for the new id
    BatchId,         // u64 for the batch counting
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

#[contracttype]
pub struct KeyManager {
    pub user: Address,
    pub key_id: u32,
}

#[contracttype]
pub struct WithdrawRequestManager {
    pub id: u64,
}

#[contracttype]
#[derive(PartialEq, Clone)]
pub enum WithdrawStatus {
    Requested,
    Rejected,
    Executed,
}

#[contracttype]
#[derive(PartialEq)]
pub struct WithdrawData {
    pub user: Address,
    pub token: Address,
    pub amount: i128,
    pub status: WithdrawStatus,
}
