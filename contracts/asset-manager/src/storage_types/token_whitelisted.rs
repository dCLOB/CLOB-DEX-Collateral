use super::TokenWhitelisted;
use soroban_sdk::{Address, Env};

impl TokenWhitelisted {
    pub fn new(token: Address) -> Self {
        Self { token }
    }

    pub fn is_token_whitelisted(&self, e: &Env) -> bool {
        e.storage().instance().get(self).unwrap_or_default()
    }

    pub fn set_token_whitelisted(&self, e: &Env, value: bool) {
        e.storage().instance().set(self, &value);
    }
}
