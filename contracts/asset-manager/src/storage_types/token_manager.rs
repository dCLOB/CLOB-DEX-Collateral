use super::{ListingStatus, TokenManager};
use crate::Error;
use soroban_sdk::{assert_with_error, Address, Env, Symbol};

impl TokenManager {
    pub fn new(token: Address) -> Self {
        Self { token }
    }

    pub fn is_listed(&self, e: &Env) -> bool {
        e.storage()
            .instance()
            .get::<_, ListingStatus>(self)
            .map_or(false, |status| matches!(status, ListingStatus::Listed))
    }

    pub fn set_listing_status(&self, e: &Env, status: &ListingStatus) {
        if let Some(stored_value) = e.storage().instance().get::<_, ListingStatus>(self) {
            // check for the same value have been already stored
            // it's cheaper in gas to assert than rewrite the value
            assert_with_error!(e, *status != stored_value, Error::ErrSameValueStored);
        }

        e.storage().instance().set(self, status);
    }

    pub fn emit_listing_status(&self, e: &Env, status: ListingStatus) {
        let topics = (Symbol::new(e, "token_listing"), &self.token);
        e.events().publish(topics, status);
    }
}
