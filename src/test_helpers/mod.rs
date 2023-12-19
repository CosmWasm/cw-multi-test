#![cfg(test)]

use cosmwasm_std::CustomMsg;
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename = "snake_case")]
pub enum CustomHelperMsg {
    SetName { name: String },
    SetAge { age: u32 },
}

impl CustomMsg for CustomHelperMsg {}

/// Persisted counter for testing purposes.
pub const COUNT: Item<u32> = Item::new("count");
