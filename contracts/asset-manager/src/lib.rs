#![no_std]
use crate::{
    error::Error,
    events::{emit_deposit, emit_withdraw},
    storage_types::{pair_manager::PairStorageInfo, DataKey},
};
use soroban_sdk::{contract, contractimpl, token, Address, Env, String};
use storage_types::ListingStatus;

mod error;
mod events;
mod storage_types;
mod test;
mod test_utils;

fn get_owner(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<_, Address>(&DataKey::Owner)
        .expect("not initialized")
}

fn get_operator_manager(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<_, Address>(&DataKey::OperatorManager)
        .expect("not initialized")
}

#[contract]
struct AssetManager;

#[contractimpl]
#[allow(clippy::needless_pass_by_value)]
impl AssetManager {
    pub fn initialize(e: Env, owner: Address, operator_manager: Address) {
        assert!(
            !e.storage().instance().has(&DataKey::Owner),
            "already initialized"
        );

        e.storage().instance().set(&DataKey::Owner, &owner);
        e.storage()
            .instance()
            .set(&DataKey::OperatorManager, &operator_manager);
    }

    pub fn owner(e: Env) -> Address {
        get_owner(&e)
    }

    pub fn operator_manager(e: Env) -> Address {
        get_operator_manager(&e)
    }

    pub fn set_token_status(e: Env, token: Address, status: ListingStatus) {
        let owner = get_owner(&e);
        owner.require_auth();

        let token_whitelisted = storage_types::TokenManager::new(token);

        token_whitelisted.set_listing_status(&e, status);

        //TODO emit event
    }

    pub fn set_pair_status(
        e: Env,
        symbol: String,
        pair: (Address, Address),
        status: ListingStatus,
    ) {
        let owner = get_owner(&e);
        owner.require_auth();

        let pair_manager = storage_types::PairManager::new(symbol);

        let pair_info = PairStorageInfo::new(pair, status);
        pair_manager.set_pair_info(&e, &pair_info);

        // TODO emit event
    }

    pub fn balance(e: Env, user: Address, token: Address) -> i128 {
        storage_types::UserBalanceManager::new(user, token).read_user_balance(&e)
    }

    pub fn deposit(e: Env, user: Address, token: Address, amount: i128) {
        user.require_auth();
        assert!(amount > 0, "amount must be positive");

        let token_whitelisted = storage_types::TokenManager::new(token.clone());
        assert!(token_whitelisted.is_listed(&e), "token is not whitelisted");

        let client = token::Client::new(&e, &token);
        client.transfer(&user, &e.current_contract_address(), &amount);

        let user_balance = storage_types::UserBalanceManager::new(user.clone(), token.clone());
        let balance = user_balance.read_user_balance(&e);
        user_balance.write_user_balance(&e, balance + amount);

        emit_deposit(&e, &user, &token, amount);
    }

    pub fn withdraw(e: Env, user: Address, token: Address, amount: i128) {
        user.require_auth();
        assert!(amount > 0, "amount must be positive");

        let user_balance = storage_types::UserBalanceManager::new(user.clone(), token.clone());
        let balance = user_balance.read_user_balance(&e);

        assert!(balance >= amount, "balance not enough to withdraw");
        user_balance.write_user_balance(&e, balance - amount);

        let client = token::Client::new(&e, &token);
        client.transfer(&e.current_contract_address(), &user, &amount);

        // emit events
        emit_withdraw(&e, &user, &token, amount);
    }
}
