use soroban_sdk::{Address, Env, Symbol};

pub(crate) fn emit_deposit(e: &Env, user: &Address, token: &Address, amount: i128) {
    let topics = (Symbol::new(e, "deposit"), user, token);
    e.events().publish(topics, amount);
}

pub(crate) fn emit_withdraw(e: &Env, user: &Address, token: &Address, amount: i128) {
    let topics = (Symbol::new(e, "withdraw"), user, token);
    e.events().publish(topics, amount);
}
