use crate::types::trade_upload::TradeUploadData;
use soroban_sdk::{contracttype, Address, Bytes, BytesN};
pub(crate) mod trade_upload;

#[contracttype]
pub struct ValidateUserSignatureData {
    pub user: Address,
    pub key_id: u32,
    pub message: Bytes,
    pub signature: BytesN<64>,
}

#[contracttype]
pub struct ExecutionWithdrawData {
    pub id: u64,
    pub user: Address,
    pub token: Address,
    pub amount: i128,
    pub execution_status: OperatorWithdrawStatus,
}

#[contracttype]
pub enum OperatorAction {
    ValidateUserSignature(ValidateUserSignatureData),
    ExecuteWithdraw(ExecutionWithdrawData),
    TradeUpload(TradeUploadData),
}

#[contracttype]
pub enum OperatorWithdrawStatus {
    Approve,
    Reject,
}
