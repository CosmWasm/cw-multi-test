#![cfg(test)]

use cosmwasm_schema::cw_serde;
use cosmwasm_std::CustomMsg;

pub mod caller;
pub mod echo;
pub mod error;
pub mod gov;
pub mod hackatom;
pub mod ibc;
pub mod payout;
pub mod reflect;
pub mod stargate;

/// Custom message for testing purposes.
#[cw_serde]
#[derive(Default)]
#[serde(rename = "snake_case")]
pub enum CustomHelperMsg {
    SetName {
        name: String,
    },
    SetAge {
        age: u32,
    },
    #[default]
    NoOp,
}

impl CustomMsg for CustomHelperMsg {}
