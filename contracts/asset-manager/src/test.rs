// #![cfg(test)]

use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use soroban_sdk::{
    testutils::{Address as AddressTestTrait, Ledger},
    token, Address, Bytes, BytesN, Env, String,
};

use crate::{
    storage_types::{
        user_balance_manager::UserBalances, ExecutionWithdrawData, ListingStatus, OperatorAction,
        OperatorWithdrawStatus, ValidateUserSignatureData,
    },
    test_utils::{register_test_contract, AssetManager},
};

fn create_asset_manager_contract(
    e: &Env,
    owner: &Address,
    operator: &Address,
) -> (Address, AssetManager) {
    let id = register_test_contract(e);
    let asset_manager = AssetManager::new(e, id.clone());
    asset_manager.client().initialize(owner, &operator);
    (id, asset_manager)
}

fn advance_ledger(e: &Env, delta: u64) {
    e.ledger().with_mut(|l| {
        l.timestamp += delta;
    });
}

struct Setup<'a> {
    env: Env,
    owner: Address,
    operator: Address,
    user1: Address,
    token: token::Client<'a>,
    asset_manager: AssetManager,
    asset_manager_id: Address,
}

impl<'a> Setup<'a> {
    pub fn with_default_listed_token(&self) -> &Self {
        self.asset_manager
            .client()
            .mock_all_auths()
            .set_token_status(&self.token.address, &ListingStatus::Listed);
        self
    }

    pub fn with_default_deposit(&self, amount: i128) -> &Self {
        self.asset_manager.client().mock_all_auths().deposit(
            &self.user1,
            &self.token.address,
            &amount,
        );
        self
    }
}

fn create_token_contract<'a>(
    e: &Env,
    admin: &Address,
) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
    let contract_address = e.register_stellar_asset_contract(admin.clone());
    (
        token::Client::new(e, &contract_address),
        token::StellarAssetClient::new(e, &contract_address),
    )
}

// /// Sets up a asset_manager
impl Setup<'_> {
    fn new() -> Self {
        let e: Env = soroban_sdk::Env::default();
        let owner = Address::random(&e);
        let operator = Address::random(&e);
        let user1 = Address::random(&e);

        // Create the token contract
        let token_admin = Address::random(&e);
        let (token, token_admin) = create_token_contract(&e, &token_admin);

        // Create the asset_manager contract
        let (asset_manager_id, asset_manager) =
            create_asset_manager_contract(&e, &owner, &operator);

        // Mint some tokens to work with
        token_admin.mock_all_auths().mint(&user1, &10);

        // asset_manager_id.client().mock_all_auths().deposit(&user1, &10);

        Self {
            env: e,
            owner,
            operator,
            user1,
            token,
            asset_manager,
            asset_manager_id,
        }
    }
}

#[test]
fn check_initialized() {
    let setup = Setup::new();

    assert_eq!(setup.token.balance(&setup.user1), 10);
    assert_eq!(setup.asset_manager.client().owner(), setup.owner);
    assert_eq!(
        setup.asset_manager.client().operator_manager(),
        setup.operator
    );
}

#[test]
#[should_panic(expected = "6")]
fn check_deposit_fail_for_unsupported_token() {
    let setup = Setup::new();

    setup
        .asset_manager
        .client()
        .mock_all_auths()
        .deposit(&setup.user1, &setup.token.address, &10);
}

#[test]
fn check_token_listed_delisted() {
    let setup = Setup::new();

    assert!(!setup
        .asset_manager
        .client()
        .is_token_listed(&setup.token.address));

    setup
        .asset_manager
        .client()
        .mock_all_auths()
        .set_token_status(&setup.token.address, &ListingStatus::Listed);

    assert!(setup
        .asset_manager
        .client()
        .is_token_listed(&setup.token.address));

    setup
        .asset_manager
        .client()
        .mock_all_auths()
        .set_token_status(&setup.token.address, &ListingStatus::Delisted);

    assert!(!setup
        .asset_manager
        .client()
        .is_token_listed(&setup.token.address));
}

#[test]
fn check_pair_listed_delisted() {
    let setup = Setup::new();
    let pair_symbol = String::from_slice(&setup.env, "SYMBOL");
    let (token2, _) = create_token_contract(&setup.env, &Address::random(&setup.env));

    assert!(!setup.asset_manager.client().is_pair_listed(&pair_symbol));

    setup
        .asset_manager
        .client()
        .mock_all_auths()
        .set_pair_status(
            &pair_symbol,
            &(setup.token.address.clone(), token2.address.clone()),
            &ListingStatus::Listed,
        );

    assert!(setup.asset_manager.client().is_pair_listed(&pair_symbol));

    setup
        .asset_manager
        .client()
        .mock_all_auths()
        .set_pair_status(
            &pair_symbol,
            &(setup.token.address, token2.address),
            &ListingStatus::Delisted,
        );

    assert!(!setup.asset_manager.client().is_pair_listed(&pair_symbol));
}

#[test]
fn check_deposit() {
    let setup = Setup::new();

    setup
        .asset_manager
        .client()
        .mock_all_auths()
        .set_token_status(&setup.token.address, &ListingStatus::Listed);

    setup
        .asset_manager
        .client()
        .mock_all_auths()
        .deposit(&setup.user1, &setup.token.address, &10);

    assert_eq!(
        setup
            .asset_manager
            .client()
            .balances(&setup.user1, &setup.token.address)
            .balance,
        10
    );

    assert_eq!(setup.token.balance(&setup.asset_manager_id), 10);
}

#[test]
fn check_withdraw_approved() {
    let setup = Setup::new();

    // pre-setup for withdrawal
    setup.with_default_listed_token().with_default_deposit(10);

    let id = setup
        .asset_manager
        .client()
        .mock_all_auths()
        .request_withdraw(&setup.user1, &setup.token.address, &4);

    let UserBalances {
        balance,
        balance_on_withdraw,
    } = setup
        .asset_manager
        .client()
        .balances(&setup.user1, &setup.token.address);

    assert_eq!(balance, 6);
    assert_eq!(balance_on_withdraw, 4);

    setup
        .asset_manager
        .client()
        .mock_all_auths()
        .execute_action(&OperatorAction::ExecuteWithdraw(ExecutionWithdrawData {
            id,
            user: setup.user1.clone(),
            token: setup.token.address.clone(),
            amount: 4,
            execution_status: OperatorWithdrawStatus::Approve,
        }));

    let UserBalances {
        balance,
        balance_on_withdraw,
    } = setup
        .asset_manager
        .client()
        .balances(&setup.user1, &setup.token.address);

    assert_eq!(balance, 6);
    assert_eq!(balance_on_withdraw, 0);
    assert_eq!(setup.token.balance(&setup.user1), 4);
}

#[test]
fn check_withdraw_rejected() {
    let setup = Setup::new();

    // pre-setup for withdrawal
    setup.with_default_listed_token().with_default_deposit(10);

    let id = setup
        .asset_manager
        .client()
        .mock_all_auths()
        .request_withdraw(&setup.user1, &setup.token.address, &4);

    let UserBalances {
        balance,
        balance_on_withdraw,
    } = setup
        .asset_manager
        .client()
        .balances(&setup.user1, &setup.token.address);

    assert_eq!(balance, 6);
    assert_eq!(balance_on_withdraw, 4);

    setup
        .asset_manager
        .client()
        .mock_all_auths()
        .execute_action(&OperatorAction::ExecuteWithdraw(ExecutionWithdrawData {
            id,
            user: setup.user1.clone(),
            token: setup.token.address.clone(),
            amount: 4,
            execution_status: OperatorWithdrawStatus::Reject,
        }));

    let UserBalances {
        balance,
        balance_on_withdraw,
    } = setup
        .asset_manager
        .client()
        .balances(&setup.user1, &setup.token.address);

    assert_eq!(balance, 10);
    assert_eq!(balance_on_withdraw, 0);
}

#[test]
fn check_verify_signature() {
    let setup = Setup::new();

    let mut csprng = OsRng;
    let signing_key: SigningKey = SigningKey::generate(&mut csprng);

    let verifying_key = signing_key.verifying_key().to_bytes();

    setup.asset_manager.client().user_announce_key(
        &setup.user1,
        &1,
        &BytesN::from_array(&setup.env, &verifying_key),
    );

    assert_eq!(
        setup
            .asset_manager
            .client()
            .get_user_key(&setup.user1, &1)
            .to_array(),
        verifying_key
    );

    let message: &[u8] = b"Hello world!";
    let signature = signing_key.sign(message);

    setup
        .asset_manager
        .client()
        .mock_all_auths()
        .execute_action(&OperatorAction::ValidateUserSignature(
            ValidateUserSignatureData {
                user: setup.user1,
                key_id: 1,
                message: Bytes::from_slice(&setup.env, message),
                signature: BytesN::from_array(&setup.env, &signature.to_bytes()),
            },
        )); // would panic in case the signature is not valid
}

#[test]
#[should_panic]
fn check_verify_signature_failed() {
    let setup = Setup::new();

    let mut csprng = OsRng;
    let signing_key: SigningKey = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key().to_bytes();

    setup.asset_manager.client().user_announce_key(
        &setup.user1,
        &1,
        &BytesN::from_array(&setup.env, &verifying_key),
    );

    assert_eq!(
        setup
            .asset_manager
            .client()
            .get_user_key(&setup.user1, &1)
            .to_array(),
        verifying_key
    );

    let new_signing_key: SigningKey = SigningKey::generate(&mut csprng);
    let message: &[u8] = b"Hello world!";
    let signature = new_signing_key.sign(message);

    setup
        .asset_manager
        .client()
        .mock_all_auths()
        .execute_action(&OperatorAction::ValidateUserSignature(
            ValidateUserSignatureData {
                user: setup.user1,
                key_id: 1,
                message: Bytes::from_slice(&setup.env, message),
                signature: BytesN::from_array(&setup.env, &signature.to_bytes()),
            },
        )); // would panic because the signature is made by the other key
}
