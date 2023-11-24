use crate::{
    error::Error,
    get_batch_id, increment_batch_id,
    storage_types::{self, UserBalanceManager, WithdrawStatus},
    types::{trade_upload::TradeUploadData, ExecutionWithdrawData, OperatorWithdrawStatus},
};
use soroban_sdk::{assert_with_error, token, Env, Symbol};

pub(crate) fn process_withdraw_request(e: &Env, withdraw_data: ExecutionWithdrawData) {
    let ExecutionWithdrawData {
        id,
        user,
        token,
        amount,
        execution_status,
    } = withdraw_data;

    let withdraw_request_manager = storage_types::WithdrawRequestManager::new(id);

    let mut withdraw_request = withdraw_request_manager.read_withdraw_request(e);

    assert_with_error!(
        &e,
        withdraw_request.status == WithdrawStatus::Requested,
        Error::ErrWithdrawRequestAlreadyProcessed
    );

    assert_with_error!(
        &e,
        withdraw_request.user == user
            && withdraw_request.token == token
            && withdraw_request.amount == amount,
        Error::ErrWithdrawRequestDataMismatch
    );

    let user_balance_manager = UserBalanceManager::new(user.clone(), token.clone());
    let mut balances = user_balance_manager.read_user_balance(e);

    match execution_status {
        OperatorWithdrawStatus::Approve => {
            assert_with_error!(
                &e,
                balances.balance_on_withdraw >= amount,
                Error::ErrBalanceNotEnough
            );

            balances.balance_on_withdraw -= amount;

            withdraw_request.status = WithdrawStatus::Executed;

            let client = token::Client::new(e, &token);
            client.transfer(&e.current_contract_address(), &user, &amount);
        }
        OperatorWithdrawStatus::Reject => {
            withdraw_request.status = WithdrawStatus::Rejected;

            balances.balance_on_withdraw -= amount;

            balances.balance += amount;
        }
    }

    user_balance_manager.write_user_balance(e, &balances);

    withdraw_request_manager.write_withdraw_request(e, &withdraw_request);

    withdraw_request_manager.emit_withdraw_request(e, withdraw_request);
}

pub(crate) fn process_trades_batch(e: &Env, trade_data: TradeUploadData) {
    assert_with_error!(
        e,
        trade_data.batch_id == get_batch_id(e),
        Error::ErrBatchIdNotMatch
    );

    for trade_pair in trade_data.trades {
        trade_pair.verify_signatures(e);

        trade_pair.execute_pair_swap(e);
    }

    increment_batch_id(e);
    emit_trades_batch_processed(e, trade_data.batch_id);
}

fn emit_trades_batch_processed(e: &Env, batch_id: u64) {
    let topics = (Symbol::new(e, "batch_processed"),);
    e.events().publish(topics, batch_id);
}
