// #![cfg(test)]

use soroban_sdk::{
    testutils::{Address as AddressTestTrait, Ledger},
    token, Address, Env,
};

use crate::test_utils::{register_test_contract, AssetManager};

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
fn check_deposit_withdraw() {
    let setup = Setup::new();

    setup
        .asset_manager
        .client()
        .mock_all_auths()
        .deposit(&setup.user1, &setup.token.address, &10);

    assert_eq!(
        setup
            .asset_manager
            .client()
            .balance(&setup.user1, &setup.token.address),
        10
    );

    assert_eq!(setup.token.balance(&setup.asset_manager_id), 10);

    setup
        .asset_manager
        .client()
        .mock_all_auths()
        .withdraw(&setup.user1, &setup.token.address, &5);

    assert_eq!(setup.token.balance(&setup.asset_manager_id), 5);
    assert_eq!(
        setup
            .asset_manager
            .client()
            .balance(&setup.user1, &setup.token.address),
        5
    );
    assert_eq!(setup.token.balance(&setup.user1), 5);
}
