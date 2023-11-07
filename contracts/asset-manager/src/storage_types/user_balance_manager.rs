use soroban_sdk::{Address, Env, Symbol};

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

    pub fn emit_deposit(&self, e: &Env, amount: i128) {
        let topics = (Symbol::new(e, "deposit"), &self.user, &self.token);
        e.events().publish(topics, amount);
    }

    pub fn emit_withdraw(&self, e: &Env, amount: i128) {
        let topics = (Symbol::new(e, "withdraw"), &self.user, &self.token);
        e.events().publish(topics, amount);
    }
}
