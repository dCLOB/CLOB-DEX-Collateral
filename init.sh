#!/bin/bash

set -e

NETWORK="$1"

PATH=./target/bin:$PATH

if [[ -f "./.CLOB_DEX_DAPP/asset_manager_id" ]]; then
  echo "Found existing './.CLOB_DEX_DAPP' directory; already initialized."
  exit 0
fi

if [[ -f "./target/bin/soroban" ]]; then
  echo "Using soroban binary from ./target/bin"
else
  echo "Building pinned soroban binary"
  cargo install_soroban
fi

case "$1" in
standalone)
  SOROBAN_RPC_HOST="http://localhost:8000"
  SOROBAN_RPC_URL="$SOROBAN_RPC_HOST/soroban/rpc"
  SOROBAN_NETWORK_PASSPHRASE="Standalone Network ; February 2017"
  FRIENDBOT_URL="$SOROBAN_RPC_HOST/friendbot"
  ;;
futurenet)
  SOROBAN_RPC_HOST="https://rpc-futurenet.stellar.org"
  SOROBAN_RPC_URL="$SOROBAN_RPC_HOST"
  SOROBAN_NETWORK_PASSPHRASE="Test SDF Future Network ; October 2022"
  FRIENDBOT_URL="https://friendbot-futurenet.stellar.org/"
  ;;
testnet)
  SOROBAN_RPC_HOST="https://soroban-testnet.stellar.org"
  SOROBAN_RPC_URL="$SOROBAN_RPC_HOST"
  SOROBAN_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"
  FRIENDBOT_URL="https://friendbot.stellar.org/"
  ;;
*)
  echo "Usage: $0 standalone|futurenet|testnet [rpc-host]"
  exit 1
  ;;
esac

echo "Using $NETWORK network"
echo "  RPC URL: $SOROBAN_RPC_URL"
echo "  Friendbot URL: $FRIENDBOT_URL"

echo Add the $NETWORK network to cli client
soroban config network add \
  --rpc-url "$SOROBAN_RPC_URL" \
  --network-passphrase "$SOROBAN_NETWORK_PASSPHRASE" "$NETWORK"

echo Add $NETWORK to .soroban-example-dapp for use with npm scripts
mkdir -p .CLOB_DEX_DAPP
echo $NETWORK > ./.CLOB_DEX_DAPP/network
echo $SOROBAN_RPC_URL > ./.CLOB_DEX_DAPP/rpc-url
echo "$SOROBAN_NETWORK_PASSPHRASE" > ./.CLOB_DEX_DAPP/passphrase
echo "{ \"network\": \"$NETWORK\", \"rpcUrl\": \"$SOROBAN_RPC_URL\", \"networkPassphrase\": \"$SOROBAN_NETWORK_PASSPHRASE\" }" > ./.CLOB_DEX_DAPP/config.json

if !(soroban config identity ls | grep owner 2>&1 >/dev/null); then
  echo Create the owner identity
  soroban config identity generate owner
fi

if !(soroban config identity ls | grep operator 2>&1 >/dev/null); then
  echo Create the operator identity
  soroban config identity generate operator
fi

if !(soroban config identity ls | grep fee_collector 2>&1 >/dev/null); then
  echo Create the operator identity
  soroban config identity generate fee_collector
fi

if !(soroban config identity ls | grep alice 2>&1 >/dev/null); then
  echo Create the alice identity
  soroban config identity generate alice
fi

if !(soroban config identity ls | grep bob 2>&1 >/dev/null); then
  echo Create the alice identity
  soroban config identity generate bob
fi

OWNER=$(soroban config identity address owner)

OPERATOR=$(soroban config identity address operator)

FEE_COLLECTOR=$(soroban config identity address fee_collector)

ALICE=$(soroban config identity address alice)

BOB=$(soroban config identity address bob)

# This will fail if the account already exists, but it'll still be fine.
curl --silent -X POST "$FRIENDBOT_URL?addr=$OWNER" >/dev/null
curl --silent -X POST "$FRIENDBOT_URL?addr=$OPERATOR" >/dev/null
curl --silent -X POST "$FRIENDBOT_URL?addr=$FEE_COLLECTOR" >/dev/null
curl --silent -X POST "$FRIENDBOT_URL?addr=$ALICE" >/dev/null
curl --silent -X POST "$FRIENDBOT_URL?addr=$BOB" >/dev/null

ARGS="--network $NETWORK --source owner"

echo Build contracts
rm -rf ./res

rustup target add wasm32-unknown-unknown
cargo build --all --target wasm32-unknown-unknown --release

mkdir res
cp target/wasm32-unknown-unknown/release/*.wasm ./res/

for wasm_file in ./res/*.wasm; do
    soroban contract optimize --wasm "$wasm_file" --wasm-out "$wasm_file"
done

echo Deploy the asset-manager contract
ASSET_MANAGER="$(
  soroban contract deploy $ARGS \
    --wasm res/asset_manager.wasm
)"
echo "Contract deployed succesfully with ID: $ASSET_MANAGER"
echo "$ASSET_MANAGER" > .CLOB_DEX_DAPP/asset_manager_id

echo "Initialize the asset-manager contract"
soroban contract invoke \
  $ARGS \
  --id "$ASSET_MANAGER" \
  -- \
  initialize \
  --owner "$OWNER" \
  --operator_manager "$OPERATOR" \
  --fee_collector "$FEE_COLLECTOR"


echo "Deploy the token1 contract"
TOKEN1="$(
  soroban contract deploy $ARGS \
    --wasm res/test_token_contract.wasm
)"
echo "Contract deployed succesfully with ID: $TOKEN1"
echo "$TOKEN1" > .CLOB_DEX_DAPP/token1

echo "Initialize the token1 contract"
soroban contract invoke \
  $ARGS \
  --id "$TOKEN1" \
  -- \
  initialize \
  --admin "$OWNER" \
  --decimal 18 \
  --name "TOKEN1" \
  --symbol "TKN1"

echo "Deploy the token2 contract"
TOKEN2="$(
  soroban contract deploy $ARGS \
    --wasm res/test_token_contract.wasm
)"
echo "Contract deployed succesfully with ID: $TOKEN2"
echo "$TOKEN2" > .CLOB_DEX_DAPP/token1

echo "Initialize the token2 contract"
soroban contract invoke \
  $ARGS \
  --id "$TOKEN2" \
  -- \
  initialize \
  --admin "$OWNER" \
  --decimal 18 \
  --name "TOKEN2" \
  --symbol "TKN2"

echo "List token1 to asset-manager"
soroban contract invoke \
    $ARGS \
    --id "$ASSET_MANAGER" \
    -- \
    set_token_status \
    --token "$TOKEN1" \
    --status Listed

echo "List token2 to asset-manager"
soroban contract invoke \
    $ARGS \
    --id "$ASSET_MANAGER" \
    -- \
    set_token_status \
    --token "$TOKEN2" \
    --status Listed

echo "List pair for token1 and token2"
soroban contract invoke \
    $ARGS \
    --id "$ASSET_MANAGER" \
    -- \
    set_pair_status \
    --symbol '"SPOT_TKN1_TKN2"' \
    --token1 "$TOKEN1" \
    --token2 "$TOKEN2" \
    --status Listed

echo "Minting for 100 TOKEN1 to ALICE"
soroban contract invoke \
    $ARGS \
    --id "$TOKEN1" \
    -- \
    mint \
    --to "$ALICE" \
    --amount 100

echo "Minting for 100 TOKEN2 to BOB"
soroban contract invoke \
    $ARGS \
    --id "$TOKEN2" \
    -- \
    mint \
    --to "$BOB" \
    --amount 100

echo "Deposit from alice to the asset-manager"
soroban contract invoke \
    --network "$NETWORK" \
    --source alice \
    --id "$ASSET_MANAGER" \
    -- \
    deposit \
    --user "$ALICE" \
    --token "$TOKEN1" \
    --amount 100

echo "Deposit from bob to the asset-manager"
soroban contract invoke \
    --network "$NETWORK" \
    --source bob \
    --id "$ASSET_MANAGER" \
    -- \
    deposit \
    --user "$BOB" \
    --token "$TOKEN2" \
    --amount 100

echo "Done"
