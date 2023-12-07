#![no_std]
use crate::{
    error::Error,
    storage_types::{pair_manager::PairStorageInfo, DataKey, WithdrawData, WithdrawStatus},
};
use operator_handlers::{process_trades_batch, process_withdraw_request};
use soroban_sdk::{
    assert_with_error, contract, contractimpl, panic_with_error, token, Address, BytesN, Env,
    String,
};
use storage_types::{user_balance_manager::UserBalances, ListingStatus};
use types::{OperatorAction, ValidateUserSignatureData};

mod error;
mod operator_handlers;
mod storage_types;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod test_utils;
mod types;

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

fn get_fee_collector(e: &Env) -> Address {
    if let Some(fee_collector) = e
        .storage()
        .instance()
        .get::<_, Address>(&DataKey::FeeCollector)
    {
        fee_collector
    } else {
        panic_with_error!(&e, Error::ErrNotInitialized)
    }
}

fn get_new_withdraw_id(e: &Env) -> u64 {
    let key = DataKey::WithdrawId;
    let id = e.storage().instance().get::<_, u64>(&key).unwrap();
    e.storage().instance().set(&key, &(id + 1));
    id
}

fn get_batch_id(e: &Env) -> u64 {
    e.storage()
        .instance()
        .get::<_, u64>(&DataKey::BatchId)
        .unwrap()
}

fn increment_batch_id(e: &Env) {
    let current_batch_id = get_batch_id(e);
    e.storage()
        .instance()
        .set(&DataKey::BatchId, &(current_batch_id + 1))
}

#[contract]
struct AssetManager;

#[contractimpl]
#[allow(clippy::needless_pass_by_value)]
impl AssetManager {
    pub fn initialize(e: Env, owner: Address, operator_manager: Address, fee_collector: Address) {
        assert_with_error!(
            &e,
            !e.storage().instance().has(&DataKey::Owner),
            Error::ErrAlreadyInitialized
        );

        e.storage().instance().set(&DataKey::Owner, &owner);
        e.storage()
            .instance()
            .set(&DataKey::OperatorManager, &operator_manager);
        e.storage()
            .instance()
            .set(&DataKey::FeeCollector, &fee_collector);
        e.storage()
            .instance()
            .set::<DataKey, u64>(&DataKey::WithdrawId, &1);
        e.storage()
            .instance()
            .set::<DataKey, u64>(&DataKey::BatchId, &1);
    }

    pub fn owner(e: Env) -> Address {
        get_owner(&e)
    }

    pub fn operator_manager(e: Env) -> Address {
        get_operator_manager(&e)
    }

    pub fn fee_collector(e: Env) -> Address {
        get_fee_collector(&e)
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
        token1: Address,
        token2: Address,
        status: ListingStatus,
    ) {
        let owner = get_owner(&e);
        owner.require_auth();

        assert_with_error!(
            &e,
            storage_types::TokenManager::new(token1.clone()).is_listed(&e),
            Error::ErrTokenIsNotListed
        );

        assert_with_error!(
            &e,
            storage_types::TokenManager::new(token2.clone()).is_listed(&e),
            Error::ErrTokenIsNotListed
        );

        let pair_manager = storage_types::PairManager::new(symbol);

        let pair_info = PairStorageInfo::new((token1, token2), status.clone());
        pair_manager.set_pair_info(&e, &pair_info);

        pair_manager.emit_listing_status(&e, pair_info.get_pair(), status);
    }

    pub fn balances(e: Env, user: Address, token: Address) -> UserBalances {
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
        let mut balances = user_balance_manager.read_user_balance(&e);
        balances.balance += amount;
        user_balance_manager.write_user_balance(&e, &balances);

        user_balance_manager.emit_deposit(&e, amount);
    }

    pub fn request_withdraw(e: Env, user: Address, token: Address, amount: i128) -> u64 {
        user.require_auth();
        assert_with_error!(&e, amount > 0, Error::ErrAmountMustBePositive);

        let user_balance_manager =
            storage_types::UserBalanceManager::new(user.clone(), token.clone());
        let mut balances = user_balance_manager.read_user_balance(&e);

        assert_with_error!(e, balances.balance >= amount, Error::ErrBalanceNotEnough);
        balances.balance -= amount;
        balances.balance_on_withdraw += amount;
        user_balance_manager.write_user_balance(&e, &balances);

        let new_id = get_new_withdraw_id(&e);
        let withdraw_manager = storage_types::WithdrawRequestManager::new(new_id);

        let withdraw_request_data = WithdrawData {
            token,
            amount,
            status: WithdrawStatus::Requested,
            user,
        };

        user_balance_manager.write_user_balance(&e, &balances);
        withdraw_manager.write_withdraw_request(&e, &withdraw_request_data);

        withdraw_manager.emit_withdraw_request(&e, withdraw_request_data);

        new_id
    }

    pub fn user_announce_key(e: Env, user: Address, key_id: u32, public_key: BytesN<32>) {
        user.require_auth();

        let user_key_manager = storage_types::KeyManager::new(user, key_id);

        user_key_manager.write_public_key(&e, &public_key);
        user_key_manager.emit_announce_key_event(&e, public_key);
    }

    pub fn get_user_key(e: Env, user: Address, key_id: u32) -> BytesN<32> {
        let user_key_manager = storage_types::KeyManager::new(user, key_id);
        user_key_manager.read_public_key(&e)
    }

    pub fn execute_action(e: Env, action: OperatorAction) {
        let operator_manager = get_operator_manager(&e);
        operator_manager.require_auth();

        match action {
            OperatorAction::ValidateUserSignature(data) => {
                let ValidateUserSignatureData {
                    user,
                    key_id,
                    message,
                    signature,
                } = data;
                let key_manager = storage_types::KeyManager::new(user, key_id);
                let public_key = key_manager.read_public_key(&e);
                e.crypto().ed25519_verify(&public_key, &message, &signature);
            }
            OperatorAction::ExecuteWithdraw(execution_withdraw_data) => {
                process_withdraw_request(&e, execution_withdraw_data);
            }
            OperatorAction::TradeUpload(trade_unit_data) => {
                process_trades_batch(&e, trade_unit_data)
            }
        }
    }
}
