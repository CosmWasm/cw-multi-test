//! # Definitions enabled or disabled using crate features

#[cfg(feature = "stargate")]
pub use cosmwasm_std::GovMsg;

#[cfg(not(feature = "stargate"))]
pub use cosmwasm_std::Empty as GovMsg;
