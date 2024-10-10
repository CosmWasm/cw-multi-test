# CosmWasm MultiTest

[![cw-multi-test on crates.io][crates-badge]][crates-url]
[![docs][docs-badge]][docs-url]
[![codecov][codecov-badge]][codecov-url]
[![license][apache-badge]][apache-url]

[crates-badge]: https://img.shields.io/crates/v/cw-multi-test.svg
[crates-url]: https://crates.io/crates/cw-multi-test
[docs-badge]: https://docs.rs/cw-multi-test/badge.svg
[docs-url]: https://docs.rs/cw-multi-test
[codecov-badge]: https://codecov.io/gh/CosmWasm/cw-multi-test/branch/main/graph/badge.svg?token=IYY72ZVS3X
[codecov-url]: https://codecov.io/gh/CosmWasm/cw-multi-test
[apache-badge]: https://img.shields.io/badge/License-Apache%202.0-blue.svg
[apache-url]: LICENSE
[notice-url]: NOTICE

**Testing tools for multi-contract interactions**

## Introduction

**CosmWasm MultiTest** is a suite of testing tools designed for facilitating multi-contract
interactions within the [CosmWasm](https://github.com/CosmWasm) ecosystem.
Its primary focus is on providing developers with a robust framework for simulating
complex contract interactions and bank operations.

## Library Capabilities

**CosmWasm MultiTest** enables comprehensive unit testing, including scenarios where contracts
call other contracts and interact with several modules like bank and staking. Its current implementation
effectively handles these interactions, providing a realistic testing environment for contract developers.
The team is committed to extending **CosmWasm MultiTest**'s capabilities, making it a versatile tool
for various blockchain interaction tests.

## Feature flags

**CosmWasm MultiTest** library provides several feature flags that can be enabled like shown below:

```toml
[dev-dependencies]
cw-multi-test = { version = "2", features = ["staking", "stargate", "cosmwasm_2_1"] }
```

Since version 2.1.0, **CosmWasm MultiTest** has no default features enabled.
The table below summarizes all available features:

| Feature          | Description                                                                                        |
|------------------|----------------------------------------------------------------------------------------------------|
| **backtrace**    | Enables `backtrace` feature in **anyhow** dependency.                                              |
| **staking**      | Enables `staking` feature in **cosmwasm-std** dependency.                                          |
| **stargate**     | Enables `stargate` feature in **cosmwasm-std** dependency.                                         |
| **cosmwasm_1_1** | Enables `cosmwasm_1_1` feature in **cosmwasm-std** dependency.                                     |
| **cosmwasm_1_2** | Enables `cosmwasm_1_1` in **MultiTest** and `cosmwasm_1_2` feature in **cosmwasm-std** dependency. |
| **cosmwasm_1_3** | Enables `cosmwasm_1_2` in **MultiTest** and `cosmwasm_1_3` feature in **cosmwasm-std** dependency. |
| **cosmwasm_1_4** | Enables `cosmwasm_1_3` in **MultiTest** and `cosmwasm_1_4` feature in **cosmwasm-std** dependency. |
| **cosmwasm_2_0** | Enables `cosmwasm_1_4` in **MultiTest** and `cosmwasm_2_0` feature in **cosmwasm-std** dependency. |
| **cosmwasm_2_1** | Enables `cosmwasm_2_0` in **MultiTest** and `cosmwasm_2_1` feature in **cosmwasm-std** dependency. |

## Conclusion

**CosmWasm MultiTest** stands as a vital development tool in
the [CosmWasm](https://github.com/CosmWasm) ecosystem, especially for developers engaged
in building complex decentralized applications. As the framework evolves, it is poised to become
an even more integral part of the [CosmWasm](https://github.com/CosmWasm) development toolkit.
Users are encouraged to stay updated with its progress and contribute to its development.

## License

Licensed under [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)
(see [LICENSE][apache-url] and [NOTICE][notice-url]).

Any contribution intentionally submitted for inclusion in this crate by you,
shall be licensed as above, without any additional terms or conditions.
