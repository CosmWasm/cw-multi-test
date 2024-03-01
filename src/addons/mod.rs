//! # Additional testing tools (extensions)
//!
//! **CosmWasm MultiTest** addons provide additional tools for testing smart contracts,
//! simulating complex blockchain scenarios that developers might encounter.
//! They enhance the CosmWasm environment, enabling more advanced and nuanced testing.

mod addresses;
mod api;

pub use addresses::mock::MockAddressGenerator;
pub use api::b32::MockApiBech32;
pub use api::b32m::MockApiBech32m;
