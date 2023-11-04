use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum Error {
    ErrFinalized = 1,
    ErrSameValueStored = 2,
    ErrChangingPair = 3,
}
