use soroban_sdk::{assert_with_error, panic_with_error, Env, Symbol};

use crate::error::Error;

use super::{WithdrawData, WithdrawRequestManager, USER_DATA_BUMP_AMOUNT};

impl WithdrawRequestManager {
    pub fn new(id: u64) -> Self {
        Self { id }
    }

    pub fn read_withdraw_request(&self, e: &Env) -> WithdrawData {
        if let Some(data) = e.storage().persistent().get::<_, WithdrawData>(self) {
            e.storage()
                .persistent()
                .bump(self, USER_DATA_BUMP_AMOUNT, USER_DATA_BUMP_AMOUNT);
            data
        } else {
            panic_with_error!(e, Error::ErrWithdrawDataNotExist)
        }
    }

    pub fn write_withdraw_request(&self, e: &Env, withdraw_data: &WithdrawData) {
        if let Some(data) = e.storage().persistent().get::<_, WithdrawData>(self) {
            assert_with_error!(e, data != *withdraw_data, Error::ErrSameWithdrawDataExist);
        }
        e.storage().persistent().set(self, withdraw_data);
    }

    pub fn emit_withdraw_request(&self, e: &Env, withdraw_data: WithdrawData) {
        let topics = (Symbol::new(e, "withdraw_request"), &withdraw_data.token);
        e.events().publish(
            topics,
            (self.id, withdraw_data.amount, withdraw_data.status),
        );
    }
}
