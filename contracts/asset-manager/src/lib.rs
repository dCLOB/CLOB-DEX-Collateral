#![no_std]
use crate::{
    events::{emit_deposit, emit_withdraw},
    storage_types::DataKey,
};
use soroban_sdk::{contract, contractimpl, token, Address, Env};

mod events;
mod storage_types;
mod test;
mod testutils;

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

    pub fn whitelist_token(e: Env, token: Address, value: bool) {
        let owner = get_owner(&e);
        owner.require_auth();

        let token_whitelisted = storage_types::TokenWhitelisted::new(token);

        let is_whitelisted = token_whitelisted.is_token_whitelisted(&e);
        assert!(is_whitelisted != value, "Same value is already set");

        token_whitelisted.set_token_whitelisted(&e, value);
    }

    pub fn balance(e: Env, user: Address, token: Address) -> i128 {
        storage_types::UserBalance::new(user, token).read_user_balance(&e)
    }

    pub fn deposit(e: Env, user: Address, token: Address, amount: i128) {
        user.require_auth();
        assert!(amount > 0, "amount must be positive");

        let client = token::Client::new(&e, &token);
        client.transfer(&user, &e.current_contract_address(), &amount);

        let user_balance = storage_types::UserBalance::new(user.clone(), token.clone());
        let balance = user_balance.read_user_balance(&e);
        user_balance.write_user_balance(&e, balance + amount);

        emit_deposit(&e, &user, &token, amount);
    }

    pub fn withdraw(e: Env, user: Address, token: Address, amount: i128) {
        user.require_auth();
        assert!(amount > 0, "amount must be positive");

        let user_balance = storage_types::UserBalance::new(user.clone(), token.clone());
        let balance = user_balance.read_user_balance(&e);

        assert!(balance >= amount, "balance not enough to withdraw");
        user_balance.write_user_balance(&e, balance - amount);

        let client = token::Client::new(&e, &token);
        client.transfer(&e.current_contract_address(), &user, &amount);

        // emit events
        emit_withdraw(&e, &user, &token, amount);
    }
}
