//! prepare docs

mod addresses;
mod api;

pub use addresses::mock::MockAddressGenerator;
pub use api::bech32::MockApiBech32;
pub use api::bech32m::MockApiBech32m;
