//! # Addons
//!

mod addresses;
mod api;

pub use addresses::mock::MockAddressGenerator;
pub use api::bech32::MockApiBech32;
