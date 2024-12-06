use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum Error {
    EmptyNodeView = 1,
    ZeroValueInsert = 2,
    NotAChildOfItsParent = 3,
    NotAParentOfChild = 4,
    IncorrectPriceLevelStorageState = 5,
    InvalidOrderId = 6,
    SameValueStored = 7,
    AmountMustBePositive = 8,
    SamePairTokens = 9,
    BalanceNotEnough = 10,
    OrderNotFound = 11,
    IncorrectPrecisionCalculation = 12,
    InvalidIdFailedToRemove = 13,
    InvalidIdFailedToUpdate = 14,
    InvalidIdFailedToLoad = 15,
    PriceStoreInvalidIndex = 16,
    PriceStoreOrderNotFoundByIndex = 17,
    LevelsStorePriceNotFound = 18,
    LevelsStoreLevelNotFound = 19,
    LevelsStoreRemoveFailed = 20,
}
