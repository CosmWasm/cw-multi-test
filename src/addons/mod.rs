//! # MultiTest add-ons
//!
//! Additional components and functionalities used to enhance
//! or customize tests of CosmWasm smart contracts.

mod addresses;
mod api;

pub use addresses::mock::MockAddressGenerator;
pub use api::bech32::MockApiBech32;
pub use api::bech32m::MockApiBech32m;
