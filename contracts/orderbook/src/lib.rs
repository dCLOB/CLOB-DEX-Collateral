#![no_std]
use order_statistic_tree::Tree;
use soroban_sdk::{contract, contractimpl, contracttype, Env};

mod error;
mod order_statistic_tree;

#[contract]
pub struct Orderbook;

#[contracttype]
enum Positions {
    Long,
    Short,
}

#[contractimpl]
impl Orderbook {
    pub fn initialize(env: Env) {
        env.storage()
            .instance()
            .set(&Positions::Long, &Tree::new(&env));

        env.storage()
            .instance()
            .set(&Positions::Short, &Tree::new(&env));
    }
}
