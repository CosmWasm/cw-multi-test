//! Multitest is a design to simulate a blockchain environment in pure Rust.
//! This allows us to run unit tests that involve contract -> contract,
//! and contract -> bank interactions. This is not intended to be a full blockchain app
//! but to simulate the Cosmos SDK x/wasm module close enough to gain confidence in
//! multi-contract deployments before testing them on a live blockchain.
//!
//! To understand the design of this module, please refer to `../DESIGN.md`

mod addresses;
mod app;
mod app_builder;
mod bank;
mod checksums;
#[allow(clippy::type_complexity)]
mod contracts;
pub mod custom_handler;
pub mod error;
mod executor;
mod gov;
mod ibc;
mod module;
mod prefixed_storage;
mod staking;
mod test_helpers;
mod tests;
mod transactions;
mod wasm;

pub use crate::addresses::{AddressGenerator, SimpleAddressGenerator};
pub use crate::app::{custom_app, next_block, App, BasicApp, CosmosRouter, Router, SudoMsg};
pub use crate::app_builder::{AppBuilder, BasicAppBuilder};
pub use crate::bank::{Bank, BankKeeper, BankSudo};
pub use crate::checksums::ChecksumGenerator;
pub use crate::contracts::{Contract, ContractWrapper};
pub use crate::executor::{AppResponse, Executor};
pub use crate::gov::Gov;
pub use crate::ibc::{Ibc, IbcAcceptingModule};
pub use crate::module::{FailingModule, Module};
pub use crate::staking::{
    Distribution, DistributionKeeper, StakeKeeper, Staking, StakingInfo, StakingSudo,
};
pub use crate::wasm::{ContractData, Wasm, WasmKeeper, WasmSudo};
