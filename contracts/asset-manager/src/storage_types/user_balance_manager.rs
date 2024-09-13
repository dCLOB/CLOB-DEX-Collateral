use super::{UserBalanceManager, PERSISTENT_THRESHOLD, USER_DATA_BUMP_AMOUNT};
use soroban_sdk::{contracttype, Address, Env, Symbol};

#[contracttype]
pub struct UserBalances {
    pub balance: i128,
    pub balance_on_withdraw: i128,
}

impl UserBalanceManager {
    pub fn new(user: Address, token: Address) -> Self {
        Self { user, token }
    }

    pub fn read_user_balance(&self, e: &Env) -> UserBalances {
        if let Some(balance) = e.storage().persistent().get::<_, UserBalances>(self) {
            e.storage()
                .persistent()
                .extend_ttl(self, PERSISTENT_THRESHOLD, USER_DATA_BUMP_AMOUNT);

            balance
        } else {
            UserBalances {
                balance: 0,
                balance_on_withdraw: 0,
            }
        }
    }

    pub fn write_user_balance(&self, e: &Env, balances: &UserBalances) {
        e.storage().persistent().set(self, balances);
        // e.storage().persistent()
        // .bump(self, USER_DATA_BUMP_AMOUNT, USER_DATA_BUMP_AMOUNT);
    }

    pub fn modify_user_balance_with<F>(&self, e: &Env, f: F)
    where
        F: FnOnce(UserBalances) -> UserBalances,
    {
        let current_balances = self.read_user_balance(e);
        let modified_balances = f(current_balances);
        self.write_user_balance(e, &modified_balances);
    }

    pub fn emit_deposit(&self, e: &Env, amount: i128) {
        let topics = (Symbol::new(e, "deposit"), &self.user, &self.token);
        e.events().publish(topics, amount);
    }
}
