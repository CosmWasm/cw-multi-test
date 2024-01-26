use crate::{AcceptingModule, FailingModule, Module};
use cosmwasm_std::{Binary, Empty};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Placeholder for stargate message attributes.
#[non_exhaustive]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StargateMsg {
    /// Stargate message type.
    pub type_url: String,
    /// Stargate message body.
    pub value: Binary,
}

/// Placeholder for stargate query attributes.
#[non_exhaustive]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StargateQuery {
    /// Fully qualified service path used for routing, e.g. custom/cosmos_sdk.x.bank.v1.Query/QueryBalance.
    pub path: String,
    /// Expected protobuf message type (not any), binary encoded.
    pub data: Binary,
}

/// Interface to module handling stargate messages and queries.
pub trait Stargate: Module<ExecT = StargateMsg, QueryT = StargateQuery, SudoT = Empty> {}

/// Always accepting stargate module.
pub type StargateAcceptingModule = AcceptingModule<StargateMsg, StargateQuery, Empty>;

impl Stargate for StargateAcceptingModule {}

/// Always accepting stargate module.
pub type StargateFailingModule = FailingModule<StargateMsg, StargateQuery, Empty>;

impl Stargate for StargateFailingModule {}
