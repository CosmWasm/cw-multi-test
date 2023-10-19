use crate::{FailingModule, Module};
use cosmwasm_std::{Binary, CosmosMsg, Empty, QueryRequest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StargateMsg {
    pub type_url: String,
    pub value: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StargateQuery {
    pub path: String,
    pub data: Binary,
}

pub trait Stargate: Module<ExecT = StargateMsg, QueryT = StargateQuery, SudoT = Empty> {}

pub type FailingStargate = FailingModule<StargateMsg, StargateQuery, Empty>;

impl Stargate for FailingStargate {}

impl<T> From<StargateMsg> for CosmosMsg<T> {
    fn from(msg: StargateMsg) -> Self {
        CosmosMsg::Stargate {
            type_url: msg.type_url,
            value: msg.value,
        }
    }
}

impl<T> From<StargateQuery> for QueryRequest<T> {
    fn from(msg: StargateQuery) -> Self {
        QueryRequest::Stargate {
            path: msg.path,
            data: msg.data,
        }
    }
}
