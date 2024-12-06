use soroban_sdk::{contracttype, Address, Env, Symbol};

use crate::{PERSISTENT_THRESHOLD, USER_DATA_BUMP_AMOUNT};

#[derive(Clone)]
#[contracttype]
pub struct UserBalanceManager {
    pub user: Address,
    pub token: Address,
}

#[contracttype]
pub struct UserBalances {
    pub balance: i128,
    pub balance_in_trading: i128,
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
                balance_in_trading: 0,
            }
        }
    }

    pub fn write_user_balance(&self, e: &Env, balances: &UserBalances) {
        e.storage().persistent().set(self, balances);
        e.storage()
            .persistent()
            .extend_ttl(self, USER_DATA_BUMP_AMOUNT, USER_DATA_BUMP_AMOUNT);
    }

    pub fn modify_user_balance_with<F>(&self, e: &Env, f: F)
    where
        F: FnOnce(UserBalances) -> UserBalances,
    {
        let current_balances = self.read_user_balance(e);
        let modified_balances = f(current_balances);
        self.write_user_balance(e, &modified_balances);
    }

    pub fn emit_balance_update(&self, e: &Env, user_balances: UserBalances) {
        let topics = (Symbol::new(e, "BalanceUpdated"), &self.user, &self.token);
        e.events().publish(topics, user_balances);
    }

    pub fn emit_deposit(&self, e: &Env, amount: i128) {
        let topics = (Symbol::new(e, "Deposit"), &self.user, &self.token);
        e.events().publish(topics, amount);
    }

    pub fn emit_withdraw(&self, e: &Env, amount: i128) {
        let topics = (Symbol::new(e, "Withdraw"), &self.user, &self.token);
        e.events().publish(topics, amount);
    }
}
