## Decentralized exchange
Based on the Soroban platform for the Stellar network

## Contracts:

- Asset-manager - smart contract to store user deposits and execute orders.
- Test-token - smart contract for the token asset.

## Quick start:

> **NOTE:** Before using any other commands, please run the next commands:
>
> - Install rust
>
> ```shell
> curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
> ```
>
> - Install the wasm32-unknown-unknown target
>```
>rustup target add wasm32-unknown-unknown
>```
>
> - Install soroban cli
>
>```
>cargo install --locked --version 20.0.0-rc.4.1 soroban-cli
>```

### Compile:
```shell
cargo build
```

### Compile wasm targets
```shell
cargo build --all --target wasm32-unknown-unknown --release
```

### Run tests:
```shell
cargo test
```

## Initialization script usage
init.sh script was created in order to build, deploy and configure asset-manager. Also it deploys related fungible token contracts.
For the configuration it creates particular role based accounts.

### Example:
```shell
./init.sh ${NETWORK:-testnet | futurenet | standalone }
```

 - testnet       value argument will lead to deployment to testnet environment
 - futurenet     deploys to futurenet env
 - standalone    value provided is responsible for the localhost deployment

> **NOTE:** NETWORK variable is mandatory to bypass

> **TIP:** How to redeploy?
>
> In order to redeploy new version to the same environment or to any other it is required to remove .soroban and .CLOB_DEX_DAPP folders.
>
> To do so you can use the next command:
> ```rm -rf ./.soroban && rm -rf ./.CLOB_DEX_DAPP```
