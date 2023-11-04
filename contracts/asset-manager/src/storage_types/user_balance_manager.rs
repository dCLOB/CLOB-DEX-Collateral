use soroban_sdk::{Address, Env};

use super::{UserBalanceManager, BALANCE_BUMP_AMOUNT};

impl UserBalanceManager {
    pub fn new(user: Address, token: Address) -> Self {
        Self { user, token }
    }

    pub fn read_user_balance(&self, e: &Env) -> i128 {
        if let Some(balance) = e.storage().persistent().get::<_, i128>(self) {
            e.storage()
                .persistent()
                .bump(self, BALANCE_BUMP_AMOUNT, BALANCE_BUMP_AMOUNT);
            balance
        } else {
            0
        }
    }

    pub fn write_user_balance(&self, e: &Env, new_amount: i128) {
        e.storage().persistent().set(self, &new_amount);
        e.storage()
            .persistent()
            .bump(self, BALANCE_BUMP_AMOUNT, BALANCE_BUMP_AMOUNT);
    }
}
