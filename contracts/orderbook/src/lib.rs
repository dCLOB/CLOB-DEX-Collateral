// #![no_std]
use soroban_sdk::{contract, contractimpl, contracttype};

mod error;
mod node_impl;
mod order;
mod order_statistic_tree;
mod orderbook;
mod storage_tree;
#[cfg(test)]
mod test;

#[contract]
pub struct Orderbook;

#[contracttype]
enum Positions {
    Long,
    Short,
}

#[contractimpl]
impl Orderbook {
    //     pub fn initialize(env: Env) {
    //         env.storage()
    //             .instance()
    //             .set(&Positions::Long, &Tree::new(&env));

    //         env.storage()
    //             .instance()
    //             .set(&Positions::Short, &Tree::new(&env));
    //     }
}
