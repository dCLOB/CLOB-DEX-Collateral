use super::{KeyManager, PERSISTENT_THRESHOLD, USER_DATA_BUMP_AMOUNT};
use crate::error::Error;
use soroban_sdk::{panic_with_error, Address, BytesN, Env, Symbol};

impl KeyManager {
    pub fn new(user: Address, key_id: u32) -> Self {
        Self { user, key_id }
    }

    pub fn read_public_key(&self, e: &Env) -> BytesN<32> {
        if let Some(public_key) = e.storage().persistent().get::<_, BytesN<32>>(self) {
            e.storage()
                .persistent()
                .extend_ttl(self, PERSISTENT_THRESHOLD, USER_DATA_BUMP_AMOUNT);

            public_key
        } else {
            panic_with_error!(e, Error::ErrNoUserPublicKeyExist)
        }
    }

    pub fn write_public_key(&self, e: &Env, public_key: &BytesN<32>) {
        if e.storage()
            .persistent()
            .get::<_, BytesN<32>>(self)
            .is_some()
        {
            panic_with_error!(e, Error::ErrPublicKeyAlreadyExist)
        } else {
            e.storage().persistent().set(self, public_key);
        }
    }

    pub fn emit_announce_key_event(&self, e: &Env, public_key: BytesN<32>) {
        let topics = (Symbol::new(e, "announce_key"), &self.user);
        e.events().publish(topics, (self.key_id, public_key));
    }
}
