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
complex contract interactions and bank operations. Currently, **CosmWasm MultiTest**
is in the _alpha_ stage, and primarily used internally for testing
[cw-plus](https://github.com/CosmWasm/cw-plus) contracts. 

## Warning and Current Status 

### Alpha Stage Caution 

**CosmWasm MultiTest** is currently in its alpha development phase, primarily intended for internal use.
Developers are advised to use this framework cautiously, understanding that it is still undergoing 
significant development and refinement. **The current version's API is not stable yet**.

### Internal Use and Refinement 

Internally, the **CosmWasm MultiTest** framework is an essential tool for the
testing of cw-plus contracts. Its development is focused on ensuring the reliability 
and security of these contracts. The team is actively working on refactoring and enhancing
**CosmWasm MultiTest** to provide a more stable and feature-rich version for broader
community use in the future. 

### Framework Capabilities 

**CosmWasm MultiTest** enables comprehensive unit testing, including scenarios where contracts
call other contracts and interact with the bank module. Its current implementation 
effectively handles these interactions, providing a realistic testing environment for contract developers.
The team is committed to extending **CosmWasm MultiTest**'s capabilities, making it a versatile tool 
for various blockchain interaction tests.

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
