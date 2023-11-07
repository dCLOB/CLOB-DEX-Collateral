use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum Error {
    ErrFinalized = 1,
    ErrSameValueStored = 2,
    ErrChangingPair = 3,
    ErrSamePairTokens = 4,
    ErrAmountMustBePositive = 5,
    ErrTokenIsNotListed = 6,
    ErrBalanceNotEnough = 7,
    ErrAlreadyInitialized = 8,
    ErrNotInitialized = 9,
}
