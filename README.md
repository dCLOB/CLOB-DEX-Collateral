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