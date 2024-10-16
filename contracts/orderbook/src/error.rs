use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum Error {
    EmptyNodeView = 1,
    ZeroValueInsert = 2,
    NotAChildOfItsParent = 3,
    NotAParentOfChild = 4,
}
