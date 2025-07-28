# MultiTest

[![cw-multi-test][crates-badge]][crates-url]
[![docs][docs-badge]][docs-url]
[![codecov][codecov-badge]][codecov-url]
![coverage][coverage-badge]
[![license][apache-badge]][apache-url]

[crates-badge]: https://img.shields.io/crates/v/cw-multi-test.svg
[crates-url]: https://crates.io/crates/cw-multi-test
[docs-badge]: https://docs.rs/cw-multi-test/badge.svg
[docs-url]: https://docs.rs/cw-multi-test
[codecov-badge]: https://codecov.io/gh/CosmWasm/cw-multi-test/branch/main/graph/badge.svg
[codecov-url]: https://codecov.io/gh/CosmWasm/cw-multi-test
[coverage-badge]: https://img.shields.io/badge/coverage-F%2093.64%25%20L%2096.33%25%20R%2095.11%25-blue.svg
[apache-badge]: https://img.shields.io/badge/License-Apache%202.0-blue.svg
[apache-url]: LICENSE
[notice-url]: NOTICE
[CosmWasm]: https://github.com/CosmWasm

**Testing tools for multi-contract interactions**

## Introduction

**MultiTest** is a suite of testing tools designed for facilitating multi-contract
interactions within the [CosmWasm] ecosystem.
Its primary focus is on providing developers with a robust framework for simulating
complex contract interactions and bank operations.

## Library capabilities

CosmWasm **MultiTest** enables comprehensive unit testing, including scenarios where contracts
call other contracts and interact with several modules like bank and staking. Its current implementation
effectively handles these interactions, providing a realistic testing environment for contract developers.
The team is committed to extending CosmWasm **MultiTest**'s capabilities, making it a versatile tool
for various blockchain interaction tests.

## Feature flags

CosmWasm **MultiTest** library provides several feature flags that can be enabled like shown below:

```toml
[dev-dependencies]
cw-multi-test = { version = "3", features = ["staking", "stargate", "cosmwasm_3_0"] }
```

The table below summarizes all available features:

| Feature          | Description                                                                                        |
|------------------|----------------------------------------------------------------------------------------------------|
| **staking**      | Enables `staking` feature in **cosmwasm-std** dependency.                                          |
| **stargate**     | Enables `stargate` feature in **cosmwasm-std** dependency.                                         |
| **cosmwasm_1_1** | Enables `cosmwasm_1_1` feature in **cosmwasm-std** dependency.                                     |
| **cosmwasm_1_2** | Enables `cosmwasm_1_1` in **MultiTest** and `cosmwasm_1_2` feature in **cosmwasm-std** dependency. |
| **cosmwasm_1_3** | Enables `cosmwasm_1_2` in **MultiTest** and `cosmwasm_1_3` feature in **cosmwasm-std** dependency. |
| **cosmwasm_1_4** | Enables `cosmwasm_1_3` in **MultiTest** and `cosmwasm_1_4` feature in **cosmwasm-std** dependency. |
| **cosmwasm_2_0** | Enables `cosmwasm_1_4` in **MultiTest** and `cosmwasm_2_0` feature in **cosmwasm-std** dependency. |
| **cosmwasm_2_1** | Enables `cosmwasm_2_0` in **MultiTest** and `cosmwasm_2_1` feature in **cosmwasm-std** dependency. |
| **cosmwasm_2_2** | Enables `cosmwasm_2_1` in **MultiTest** and `cosmwasm_2_2` feature in **cosmwasm-std** dependency. |
| **cosmwasm_3_0** | Enables `cosmwasm_2_2` in **MultiTest** and `cosmwasm_3_0` feature in **cosmwasm-std** dependency. |

## Conclusion

CosmWasm **MultiTest** stands as a vital development tool in the [CosmWasm] ecosystem,
especially for developers engaged in building complex decentralized applications.
As the framework evolves, it is poised to become an even more integral part of the [CosmWasm] development toolkit.
Users are encouraged to stay updated with its progress and contribute to its development.

## License

Licensed under [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0)
(see [LICENSE][apache-url] and [NOTICE][notice-url]).

Any contribution intentionally submitted for inclusion in this crate by you,
shall be licensed as above, without any additional terms or conditions.


F: 96.55
L: 96.33
R: 95.11
