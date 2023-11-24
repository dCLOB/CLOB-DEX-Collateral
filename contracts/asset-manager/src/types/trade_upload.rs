use soroban_sdk::{assert_with_error, contracttype, Address, Bytes, BytesN, Env, String, Vec};

use crate::error::Error;
use crate::get_fee_collector;
use crate::storage_types::{self, PairManager, UserBalanceManager};

#[derive(PartialEq)]
enum PurchaseSide {
    Buy,
    Sell,
}

#[contracttype]
pub struct TradeUploadPair {
    pub buy_side: TradeUploadUnit,
    pub sell_side: TradeUploadUnit,
}

#[contracttype]
pub struct TradeUploadUnit {
    pub trade_id: u64,
    pub account: Address,
    pub symbol: String,
    // pub side: PurchaseSide,
    pub quantity: i128,
    // pub price: i128,
    pub amount: i128,
    pub fee_amount: i128,
    pub fee_token_asset: Address,
    pub timestamp: u64,
    pub order_signature: BytesN<64>,
    pub pub_key_id: u32,
    pub order: Bytes,
}

#[contracttype]
pub struct TradeUploadData {
    pub batch_id: u64,
    pub trades: Vec<TradeUploadPair>,
}

impl TradeUploadPair {
    pub fn verify_signatures(&self, e: &Env) {
        Self::verify_signature(e, &self.buy_side);
        Self::verify_signature(e, &self.sell_side);
    }

    fn verify_signature(e: &Env, trade_upload: &TradeUploadUnit) {
        let key_manager =
            storage_types::KeyManager::new(trade_upload.account.clone(), trade_upload.pub_key_id);

        let public_key = key_manager.read_public_key(e);

        e.crypto().ed25519_verify(
            &public_key,
            &trade_upload.order,
            &trade_upload.order_signature,
        );
    }

    pub fn execute_pair_swap(&self, e: &Env) {
        assert_with_error!(
            e,
            self.buy_side.symbol == self.sell_side.symbol,
            Error::ErrTradeSymbolsNotMatch
        );
        let pair_manager = PairManager::new(self.buy_side.symbol.clone());

        let pair = pair_manager.get_pair(e);

        Self::execute_trade(e, &self.buy_side, &pair, PurchaseSide::Buy);
        Self::withdraw_fee(e, &self.buy_side);

        Self::execute_trade(e, &self.sell_side, &pair, PurchaseSide::Sell);
        Self::withdraw_fee(e, &self.sell_side);
    }

    fn execute_trade(
        e: &Env,
        trade: &TradeUploadUnit,
        pair: &(Address, Address),
        side: PurchaseSide,
    ) {
        let (token_transfer, token_transfer_amount, token_deposit, token_deposit_amount) =
            match side {
                PurchaseSide::Buy => (&pair.1, trade.amount, &pair.0, trade.quantity),
                PurchaseSide::Sell => (&pair.0, trade.quantity, &pair.1, trade.amount),
            };

        let deposit_balance_manager =
            UserBalanceManager::new(trade.account.clone(), token_deposit.clone());

        deposit_balance_manager.modify_user_balance_with(e, |balances| {
            let mut balances = balances;
            balances.balance += token_deposit_amount;
            balances
        });

        let transfer_balance_manager =
            UserBalanceManager::new(trade.account.clone(), token_transfer.clone());

        transfer_balance_manager.modify_user_balance_with(e, |balances| {
            let mut balances = balances;
            assert_with_error!(
                e,
                balances.balance >= token_transfer_amount,
                Error::ErrBalanceNotEnough
            );
            balances.balance -= token_transfer_amount;
            balances
        });
    }

    fn withdraw_fee(e: &Env, trade: &TradeUploadUnit) {
        if trade.fee_amount == 0 {
            return;
        }

        let user_balance_manager =
            UserBalanceManager::new(trade.account.clone(), trade.fee_token_asset.clone());

        user_balance_manager.modify_user_balance_with(e, |balances| {
            let mut balances = balances;
            assert_with_error!(
                e,
                balances.balance >= trade.fee_amount,
                Error::ErrBalanceNotEnough
            );
            balances.balance -= trade.fee_amount;
            balances
        });

        let fee_account = get_fee_collector(e);

        let fee_collector_balance_manager =
            UserBalanceManager::new(fee_account, trade.fee_token_asset.clone());

        fee_collector_balance_manager.modify_user_balance_with(e, |balances| {
            let mut balances = balances;
            balances.balance += trade.fee_amount;
            balances
        });
    }
}
