use super::{ListingStatus, PairManager};
use crate::error::Error;
use soroban_sdk::{assert_with_error, contracttype, panic_with_error, Address, Env, String};

#[contracttype]
#[derive(PartialEq)]
pub struct PairStorageInfo {
    token1: Address,
    token2: Address,
    status: ListingStatus,
}

impl PairStorageInfo {
    pub fn new(pair: (Address, Address), status: ListingStatus) -> Self {
        Self {
            token1: pair.0,
            token2: pair.1,
            status,
        }
    }
}

impl PairManager {
    pub fn new(symbol: String) -> Self {
        Self { symbol }
    }

    pub fn is_listed(&self, e: &Env) -> bool {
        e.storage()
            .instance()
            .get::<_, PairStorageInfo>(self)
            .map_or(false, |pair_storage| {
                matches!(pair_storage.status, ListingStatus::Listed)
            })
    }

    pub fn get_pair(&self, e: &Env) -> (Address, Address) {
        if let Some(value) = e.storage().instance().get::<_, PairStorageInfo>(self) {
            (value.token1, value.token2)
        } else {
            panic_with_error!(e, Error::ErrFinalized)
        }
    }

    pub fn set_pair_info(&self, e: &Env, pair_info: &PairStorageInfo) {
        if let Some(stored_value) = e.storage().instance().get::<_, PairStorageInfo>(self) {
            // check for the same value have been already stored
            // it's cheaper in gas to assert than rewrite the value
            assert_with_error!(
                e,
                pair_info.status != stored_value.status,
                Error::ErrSameValueStored
            );

            assert_with_error!(
                e,
                pair_info.token1 == stored_value.token1 && pair_info.token2 == stored_value.token2,
                Error::ErrChangingPair
            )
        }

        assert_with_error!(
            e,
            pair_info.token1 != pair_info.token2,
            Error::ErrSamePairTokens
        );

        e.storage().instance().set(self, pair_info);
    }
}
