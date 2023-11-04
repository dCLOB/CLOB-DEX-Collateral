#![cfg(test)]

use soroban_sdk::{Address, Env};

use crate::AssetManagerClient;

pub fn register_test_contract(e: &Env) -> Address {
    e.register_contract(None, crate::AssetManager {})
}

pub struct AssetManager {
    env: Env,
    contract_id: Address,
}

impl AssetManager {
    #[must_use]
    pub fn client(&self) -> AssetManagerClient {
        AssetManagerClient::new(&self.env, &self.contract_id)
    }

    #[must_use]
    pub fn new(env: &Env, contract_id: Address) -> Self {
        Self {
            env: env.clone(),
            contract_id,
        }
    }
}
