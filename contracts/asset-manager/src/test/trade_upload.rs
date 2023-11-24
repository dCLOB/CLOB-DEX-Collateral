// #![cfg(test)]

use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use soroban_sdk::{vec, Bytes, BytesN, String};

use crate::{
    test::{Setup, DEFAULT_PAIR},
    types::{
        trade_upload::{TradeUploadData, TradeUploadPair, TradeUploadUnit},
        OperatorAction,
    },
};

#[test]
fn operator_trades_upload() {
    let setup = Setup::new();
    let initial_token_amounts = 10;

    setup
        .with_default_listed_tokens()
        .with_default_deposit(initial_token_amounts, 5)
        .with_default_listed_pair();

    let mut csprng = OsRng;
    let signing_key1: SigningKey = SigningKey::generate(&mut csprng);
    let verifying_key1 = signing_key1.verifying_key().to_bytes();

    setup.asset_manager.client().user_announce_key(
        &setup.user1,
        &1,
        &BytesN::from_array(&setup.env, &verifying_key1),
    );

    let signing_key2: SigningKey = SigningKey::generate(&mut csprng);
    let verifying_key2 = signing_key2.verifying_key().to_bytes();

    setup.asset_manager.client().user_announce_key(
        &setup.user2,
        &1,
        &BytesN::from_array(&setup.env, &verifying_key2),
    );

    let order_data = r#"
    {
        "symbol":"SPOT_TKN1_TKN2",
        "order_type":"MARKET",
    }"#;

    let order = Bytes::from_slice(&setup.env, order_data.as_bytes());

    let buy_trade = TradeUploadUnit {
        trade_id: 1,
        account: setup.user2.clone(),
        symbol: String::from_slice(&setup.env, DEFAULT_PAIR),
        quantity: 1,
        amount: 5,
        fee_amount: 1,
        fee_token_asset: setup.fee_token.address.clone(),
        timestamp: 0,
        order_signature: BytesN::from_array(
            &setup.env,
            &signing_key2.sign(order_data.as_bytes()).to_bytes(),
        ),
        pub_key_id: 1,
        order: order.clone(),
    };

    let sell_trade = TradeUploadUnit {
        trade_id: 2,
        account: setup.user1.clone(),
        symbol: String::from_slice(&setup.env, DEFAULT_PAIR),
        quantity: 1,
        amount: 5,
        fee_amount: 2,
        fee_token_asset: setup.fee_token.address.clone(),
        timestamp: 0,
        order_signature: BytesN::from_array(
            &setup.env,
            &signing_key1.sign(order_data.as_bytes()).to_bytes(),
        ),
        pub_key_id: 1,
        order,
    };

    let trade_upload_pair = TradeUploadPair {
        buy_side: buy_trade,
        sell_side: sell_trade,
    };

    let trade_upload_data = TradeUploadData {
        batch_id: 1,
        trades: vec![&setup.env, trade_upload_pair],
    };

    setup
        .asset_manager
        .client()
        .execute_action(&OperatorAction::TradeUpload(trade_upload_data));

    assert_eq!(
        setup
            .asset_manager
            .client()
            .balances(&setup.user2, &setup.token2.address)
            .balance,
        5 // initial balance = 10, side is BUY, token2 amount = 5 which were withdrawn from the main balance
    );

    assert_eq!(
        setup
            .asset_manager
            .client()
            .balances(&setup.user2, &setup.token.address)
            .balance,
        1 // initial balance = 0, side is BUY, token quantity = 1 which were deposited to the main balance
    );

    assert_eq!(
        setup
            .asset_manager
            .client()
            .balances(&setup.user1, &setup.token.address)
            .balance,
        9 // initial balance = 10, side is SELL, token quantity = 1 which were withdrawn from the main balance
    );

    assert_eq!(
        setup
            .asset_manager
            .client()
            .balances(&setup.user1, &setup.token2.address)
            .balance,
        5 // initial balance = 0, side is SELL, token2 amount = 5 which were deposited to the main balance
    );

    assert_eq!(
        setup
            .asset_manager
            .client()
            .balances(&setup.user1, &setup.fee_token.address)
            .balance,
        3 // initial balance = 5, fee = 2, expected result should be 3
    );

    assert_eq!(
        setup
            .asset_manager
            .client()
            .balances(&setup.user2, &setup.fee_token.address)
            .balance,
        4 // initial balance = 5, fee = 1, expected result should be 4
    );

    assert_eq!(
        setup
            .asset_manager
            .client()
            .balances(&setup.fee_collector, &setup.fee_token.address)
            .balance,
        3 // fee1 = 2, fee2 = 1 expected result should be 3
    );
}

#[test]
fn operator_trades_upload_without_fee() {
    let setup = Setup::new();
    let initial_token_amounts = 10;

    setup
        .with_default_listed_tokens()
        .with_default_deposit(initial_token_amounts, 5)
        .with_default_listed_pair();

    let mut csprng = OsRng;
    let signing_key1: SigningKey = SigningKey::generate(&mut csprng);
    let verifying_key1 = signing_key1.verifying_key().to_bytes();

    setup.asset_manager.client().user_announce_key(
        &setup.user1,
        &1,
        &BytesN::from_array(&setup.env, &verifying_key1),
    );

    let signing_key2: SigningKey = SigningKey::generate(&mut csprng);
    let verifying_key2 = signing_key2.verifying_key().to_bytes();

    setup.asset_manager.client().user_announce_key(
        &setup.user2,
        &1,
        &BytesN::from_array(&setup.env, &verifying_key2),
    );

    let order_data = r#"
    {
        "symbol":"SPOT_TKN1_TKN2",
        "order_type":"MARKET",
    }"#;

    let order = Bytes::from_slice(&setup.env, order_data.as_bytes());

    let buy_trade = TradeUploadUnit {
        trade_id: 1,
        account: setup.user2.clone(),
        symbol: String::from_slice(&setup.env, DEFAULT_PAIR),
        quantity: 1,
        amount: 5,
        fee_amount: 0,
        fee_token_asset: setup.fee_token.address.clone(),
        timestamp: 0,
        order_signature: BytesN::from_array(
            &setup.env,
            &signing_key2.sign(order_data.as_bytes()).to_bytes(),
        ),
        pub_key_id: 1,
        order: order.clone(),
    };

    let sell_trade = TradeUploadUnit {
        trade_id: 2,
        account: setup.user1.clone(),
        symbol: String::from_slice(&setup.env, DEFAULT_PAIR),
        quantity: 1,
        amount: 5,
        fee_amount: 0,
        fee_token_asset: setup.fee_token.address.clone(),
        timestamp: 0,
        order_signature: BytesN::from_array(
            &setup.env,
            &signing_key1.sign(order_data.as_bytes()).to_bytes(),
        ),
        pub_key_id: 1,
        order,
    };

    let trade_upload_pair = TradeUploadPair {
        buy_side: buy_trade,
        sell_side: sell_trade,
    };

    let trade_upload_data = TradeUploadData {
        batch_id: 1,
        trades: vec![&setup.env, trade_upload_pair],
    };

    setup
        .asset_manager
        .client()
        .execute_action(&OperatorAction::TradeUpload(trade_upload_data));

    assert_eq!(
        setup
            .asset_manager
            .client()
            .balances(&setup.user2, &setup.token2.address)
            .balance,
        5 // initial balance = 10, side is BUY, token2 amount = 5 which were withdrawn from the main balance
    );

    assert_eq!(
        setup
            .asset_manager
            .client()
            .balances(&setup.user2, &setup.token.address)
            .balance,
        1 // initial balance = 0, side is BUY, token quantity = 1 which were deposited to the main balance
    );

    assert_eq!(
        setup
            .asset_manager
            .client()
            .balances(&setup.user1, &setup.token.address)
            .balance,
        9 // initial balance = 10, side is SELL, token quantity = 1 which were withdrawn from the main balance
    );

    assert_eq!(
        setup
            .asset_manager
            .client()
            .balances(&setup.user1, &setup.token2.address)
            .balance,
        5 // initial balance = 0, side is SELL, token2 amount = 5 which were deposited to the main balance
    );

    assert_eq!(
        setup
            .asset_manager
            .client()
            .balances(&setup.user1, &setup.fee_token.address)
            .balance,
        5 // initial balance = 5
    );

    assert_eq!(
        setup
            .asset_manager
            .client()
            .balances(&setup.user2, &setup.fee_token.address)
            .balance,
        5 // initial balance = 5
    );

    assert_eq!(
        setup
            .asset_manager
            .client()
            .balances(&setup.fee_collector, &setup.fee_token.address)
            .balance,
        0 // no fees where attached to trades
    );
}
