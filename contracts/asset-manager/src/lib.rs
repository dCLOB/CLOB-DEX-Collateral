#![no_std]
use crate::{
    error::Error,
    storage_types::{pair_manager::PairStorageInfo, DataKey},
};
use soroban_sdk::{
    assert_with_error, contract, contractimpl, panic_with_error, token, Address, Env, String,
};
use storage_types::ListingStatus;

mod error;
mod storage_types;
mod test;
mod test_utils;

fn get_owner(e: &Env) -> Address {
    if let Some(operator_manager) = e.storage().instance().get::<_, Address>(&DataKey::Owner) {
        operator_manager
    } else {
        panic_with_error!(&e, Error::ErrNotInitialized)
    }
}

fn get_operator_manager(e: &Env) -> Address {
    if let Some(operator_manager) = e
        .storage()
        .instance()
        .get::<_, Address>(&DataKey::OperatorManager)
    {
        operator_manager
    } else {
        panic_with_error!(&e, Error::ErrNotInitialized)
    }
}

#[contract]
struct AssetManager;

#[contractimpl]
#[allow(clippy::needless_pass_by_value)]
impl AssetManager {
    pub fn initialize(e: Env, owner: Address, operator_manager: Address) {
        assert_with_error!(
            &e,
            !e.storage().instance().has(&DataKey::Owner),
            Error::ErrAlreadyInitialized
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

    pub fn is_token_listed(e: Env, token: Address) -> bool {
        let token_manager = storage_types::TokenManager::new(token);

        token_manager.is_listed(&e)
    }

    pub fn is_pair_listed(e: Env, symbol: String) -> bool {
        let pair_manager = storage_types::PairManager::new(symbol);

        pair_manager.is_listed(&e)
    }

    pub fn set_token_status(e: Env, token: Address, status: ListingStatus) {
        let owner = get_owner(&e);
        owner.require_auth();

        let token_manager = storage_types::TokenManager::new(token);

        token_manager.set_listing_status(&e, &status);

        token_manager.emit_listing_status(&e, status);
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

        let pair_info = PairStorageInfo::new(pair, status.clone());
        pair_manager.set_pair_info(&e, &pair_info);

        pair_manager.emit_listing_status(&e, pair_info.get_pair(), status);
    }

    pub fn balance(e: Env, user: Address, token: Address) -> i128 {
        storage_types::UserBalanceManager::new(user, token).read_user_balance(&e)
    }

    pub fn deposit(e: Env, user: Address, token: Address, amount: i128) {
        user.require_auth();
        assert_with_error!(&e, amount > 0, Error::ErrAmountMustBePositive);

        let token_whitelisted = storage_types::TokenManager::new(token.clone());
        assert_with_error!(
            &e,
            token_whitelisted.is_listed(&e),
            Error::ErrTokenIsNotListed
        );

        let client = token::Client::new(&e, &token);
        client.transfer(&user, &e.current_contract_address(), &amount);

        let user_balance_manager =
            storage_types::UserBalanceManager::new(user.clone(), token.clone());
        let balance = user_balance_manager.read_user_balance(&e);
        user_balance_manager.write_user_balance(&e, balance + amount);

        user_balance_manager.emit_deposit(&e, amount);
    }

    pub fn withdraw(e: Env, user: Address, token: Address, amount: i128) {
        user.require_auth();
        assert_with_error!(&e, amount > 0, Error::ErrAmountMustBePositive);

        let user_balance_manager =
            storage_types::UserBalanceManager::new(user.clone(), token.clone());
        let balance = user_balance_manager.read_user_balance(&e);

        assert_with_error!(e, balance >= amount, Error::ErrBalanceNotEnough);
        user_balance_manager.write_user_balance(&e, balance - amount);

        let client = token::Client::new(&e, &token);
        client.transfer(&e.current_contract_address(), &user, &amount);

        user_balance_manager.emit_withdraw(&e, amount);
    }
}
