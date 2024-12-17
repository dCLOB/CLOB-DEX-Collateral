use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum Error {
    IncorrectPriceLevelStorageState = 1,
    InvalidOrderId = 2,
    SameValueStored = 3,
    AmountMustBePositive = 4,
    SamePairTokens = 5,
    BalanceNotEnough = 6,
    OrderNotFound = 7,
    IncorrectPrecisionCalculation = 8,
    InvalidIdFailedToRemove = 9,
    InvalidIdFailedToUpdate = 10,
    InvalidIdFailedToLoad = 11,
    PriceStoreInvalidIndex = 12,
    PriceStoreOrderNotFoundByIndex = 13,
    LevelsStorePriceNotFound = 14,
    LevelsStoreLevelNotFound = 15,
    LevelsStoreRemoveFailed = 16,
    OrderBookNotFound = 17,
}
