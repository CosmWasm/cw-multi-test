//! # CosmWasm MultiTest
//!
//! **CosmWasm MultiTest** is designed to simulate a blockchain environment in pure Rust.
//! This allows you to run unit tests that involve **contract 🡘 contract**,
//! and **contract 🡘 module** interactions.
//!
//! **CosmWasm MultiTest** is not intended to be a full blockchain application, but to simulate
//! the Cosmos SDK modules close enough to gain confidence in multi-contract deployments,
//! before testing them on a live blockchain.
//!
//! The following sections explains some of the design for those who want to use the API,
//! as well as those who want to take a look under the hood of **CosmWasm MultiTest**.
//!
//! ## Key APIs
//!
//! ### App
//!
//! The main entry point to the system is called [App], which represents a blockchain application.
//! It maintains an idea of block height and time, which can be updated to simulate multiple
//! blocks. You can use application's [update_block](App::update_block) method to increment
//! the timestamp by 5 seconds and the height by 1 (simulating a new block) or you can write
//! any other mutator of [BlockInfo](cosmwasm_std::BlockInfo) to advance more.
//!
//! [App] exposes an entry point [execute](App::execute) that allows to execute
//! any [CosmosMsg](cosmwasm_std::CosmosMsg) and wraps it in an atomic transaction.
//! That is, only if [execute](App::execute) returns a success, then the state will be committed.
//! It returns the data and a list of [Event](cosmwasm_std::Event)s on successful execution
//! or an `Err(String)` on error. There are some helper methods tied to the [Executor] trait
//! that create the [CosmosMsg](cosmwasm_std::CosmosMsg) for you to provide a less verbose API.
//! [App]'s methods like [instantiate_contract](App::instantiate_contract),
//! [execute_contract](App::execute_contract), and [send_tokens](App::send_tokens) are exposed
//! for your convenience in writing tests.
//! Each method executes one [CosmosMsg](cosmwasm_std::CosmosMsg) atomically, as if it was submitted by a user.
//! You can also use [execute_multi](App::execute_multi) if you wish to execute multiple messages together
//! that revert the state as a whole in case of any failure.
//!
//! The other key entry point to [App] is the [Querier](cosmwasm_std::Querier) interface that it implements.
//! In particular, you can use [wrap](App::wrap) to get a [QuerierWrapper](cosmwasm_std::QuerierWrapper),
//! which provides all kinds of interesting APIs to query the blockchain, like
//! [query_balance](cosmwasm_std::QuerierWrapper::query_balances) and
//! [query_wasm_smart](cosmwasm_std::QuerierWrapper::query_wasm_smart).
//! Putting this all together, you have one [Storage](cosmwasm_std::Storage) wrapped into an application,
//! where you can execute contracts and bank, query them easily, and update the current
//! [BlockInfo](cosmwasm_std::BlockInfo), in an API that is not very verbose or cumbersome.
//! Under the hood it will process all messages returned from contracts, move _bank_ tokens
//! and call into other contracts.
//!
//! You can easily create an [App] for use in your testcode like shown below.
//! Having a single utility function for creating and configuring the [App] is the common
//! pattern while testing contracts with **MultiTest**.
//!
//! ```
//! use cw_multi_test::App;
//!
//! fn mock_app() -> App {
//!   App::default()
//! }
//! ```
//!
//! The [App] maintains the root [Storage](cosmwasm_std::Storage), and the [BlockInfo](cosmwasm_std::BlockInfo)
//! for the current block. It also contains a [Router] (discussed below), which can process
//! any [CosmosMsg](cosmwasm_std::CosmosMsg) variant by passing it to the proper keeper.
//!
//! > **Note**: [App] properly handles submessages and reply blocks.
//!
//! > **Note**: While the API currently supports custom messages, we don't currently have an implementation
//! > of the default keeper, except of experimental [CachingCustomHandler](custom_handler::CachingCustomHandler).
//!
//! ### Contracts
//!
//! Before you can call contracts, you must **instantiate** them. And to instantiate them,
//! you need a `code_id`. In `wasmd`, this `code_id` points to some stored Wasm code that is then run.
//! In **MultiTest**, we use it to point to a `Box<dyn Contract>` that should be run.
//! That is, you need to implement the [Contract] trait and then add the contract
//! to the [App] via [store_code](App::store_code) function.
//!
//! The [Contract] trait defines the major entry points to any CosmWasm contract:
//! [instantiate](Contract::instantiate), [execute](Contract::execute), [query](Contract::query),
//! [sudo](Contract::sudo), [reply](Contract::reply) (for submessages) and [migrate](Contract::migrate).
//!
//! In order to easily implement [Contract] from some existing contract code, we use the [ContractWrapper] struct,
//! which takes some function pointers and combines them. You can take a look at **test_helpers** module
//! for some examples or how to do so (and useful mocks for some test cases).
//! Here is an example of wrapping a CosmWasm contract into a [Contract] trait to be added to an [App]:
//!
//! ```
//! use cosmwasm_std::Empty;
//! use cw_multi_test::{App, Contract, ContractWrapper};
//!
//! // Contract definition.
//! mod my_contract {
//!     use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult};
//!
//!     pub fn instantiate(
//!         deps: DepsMut,
//!         env: Env,
//!         info: MessageInfo,
//!         msg: Empty,
//!     ) -> StdResult<Response> {
//!         Ok(Response::default())
//!     }
//!
//!     pub fn execute(
//!         deps: DepsMut,
//!         env: Env,
//!         info: MessageInfo,
//!         msg: Empty,
//!     ) -> StdResult<Response> {
//!         Ok(Response::default())
//!     }
//!
//!     pub fn query(deps: Deps, env: Env, msg: Empty) -> StdResult<Binary> {
//!         Ok(Binary::default())
//!     }
//! }
//!
//! // Wrapped contract.
//! pub fn contract() -> Box<dyn Contract<Empty>> {
//!     Box::new(ContractWrapper::new(
//!         my_contract::execute,
//!         my_contract::instantiate,
//!         my_contract::query,
//!     ))
//! }
//!
//! // Chain initialization.
//! let mut app = App::default();
//!
//! // Storing contract code on chain.
//! let code_id = app.store_code(contract());
//!
//! assert_eq!(1, code_id);
//!
//! // Use this `code_id` to instantiate the contract.
//! // ...
//! ```
//!
//! ### Modules
//!
//! There is only one root [Storage](cosmwasm_std::Storage), stored inside [App].
//! This is wrapped into a transaction, and then passed down to other functions to work with.
//! The code that modifies the Storage is divided into modules much like the CosmosSDK.
//! Currently, the message processing logic is divided into one module for every [CosmosMsg](cosmwasm_std) variant:
//! - [Bank] module handles [BankMsg](cosmwasm_std::BankMsg) and [BankQuery](cosmwasm_std::BankQuery) messages,
//! - [Wasm] module handles [WasmMsg](cosmwasm_std::WasmMsg) and [WasmQuery](cosmwasm_std::WasmQuery), etc.
//!
//! ### Router
//!
//! The [Router] groups all modules in the system into one "macro-module" that can handle
//! any [CosmosMsg](cosmwasm_std::CosmosMsg). While [Bank] handles [BankMsg](cosmwasm_std::BankMsg),
//! and [Wasm] handles [WasmMsg](cosmwasm_std::WasmMsg), we need to combine them into a larger composite
//! to let them process messages from [App]. This is the whole concept of the [Router].
//! If you take a look at the [execute](Router::execute) method, you will see it is quite straightforward.
//!
//! Note that the only way one module can call or query another module is by dispatching messages via the [Router].
//! This allows us to implement an independent [Wasm] in a way that it can process [SubMsg](cosmwasm_std::SubMsg)
//! that call into [Bank]. You can see an example of that in `send` method of the [WasmKeeper],
//! where it moves bank tokens from one account to another.

#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(rustdoc::missing_crate_level_docs)]

mod addresses;
mod api;
mod app;
mod app_builder;
mod bank;
mod checksums;
mod contracts;
pub mod custom_handler;
pub mod error;
mod executor;
mod featured;
mod gov;
mod ibc;
mod module;
mod prefixed_storage;
#[cfg(feature = "staking")]
mod staking;
mod stargate;
mod test_helpers;
mod tests;
mod transactions;
mod wasm;

pub use crate::addresses::{
    AddressGenerator, IntoAddr, IntoBech32, IntoBech32m, SimpleAddressGenerator,
};
pub use crate::api::{MockApiBech32, MockApiBech32m};
pub use crate::app::{
    custom_app, next_block, no_init, App, BasicApp, CosmosRouter, Router, SudoMsg,
};
pub use crate::app_builder::{AppBuilder, BasicAppBuilder};
pub use crate::bank::{Bank, BankKeeper, BankSudo};
pub use crate::checksums::ChecksumGenerator;
pub use crate::contracts::{Contract, ContractWrapper};
pub use crate::executor::{AppResponse, Executor};
pub use crate::featured::staking::{
    Distribution, DistributionKeeper, StakeKeeper, Staking, StakingInfo, StakingSudo,
};
pub use crate::gov::{Gov, GovAcceptingModule, GovFailingModule};
pub use crate::ibc::{Ibc, IbcAcceptingModule, IbcFailingModule};
pub use crate::module::{AcceptingModule, FailingModule, Module};
pub use crate::stargate::{Stargate, StargateAccepting, StargateFailing};
pub use crate::wasm::{ContractData, Wasm, WasmKeeper, WasmSudo};
