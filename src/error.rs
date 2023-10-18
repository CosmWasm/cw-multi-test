pub use anyhow::{anyhow, bail, Context as AnyContext, Error as AnyError, Result as AnyResult};
use cosmwasm_std::{WasmMsg, WasmQuery};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("Empty attribute key. Value: {value}")]
    EmptyAttributeKey { value: String },

    #[error("Empty attribute value. Key: {key}")]
    EmptyAttributeValue { key: String },

    #[error("Attribute key starts with reserved prefix _: {0}")]
    ReservedAttributeKey(String),

    #[error("Event type too short: {0}")]
    EventTypeTooShort(String),

    #[error("Unsupported wasm query: {0:?}")]
    UnsupportedWasmQuery(WasmQuery),

    #[error("Unsupported wasm message: {0:?}")]
    UnsupportedWasmMsg(WasmMsg),

    #[error("code id: invalid")]
    InvalidCodeId,

    #[error("code id {0}: no such code")]
    UnregisteredCodeId(u64),

    #[error("Address generator failure: {0}")]
    AddressGeneratorFailure(String),

    #[error("Contract with this address already exists: {0}")]
    DuplicatedContractAddress(String),
}

impl Error {
    pub fn empty_attribute_key(value: impl Into<String>) -> Self {
        Self::EmptyAttributeKey {
            value: value.into(),
        }
    }

    pub fn empty_attribute_value(key: impl Into<String>) -> Self {
        Self::EmptyAttributeValue { key: key.into() }
    }

    pub fn reserved_attribute_key(key: impl Into<String>) -> Self {
        Self::ReservedAttributeKey(key.into())
    }

    pub fn event_type_too_short(ty: impl Into<String>) -> Self {
        Self::EventTypeTooShort(ty.into())
    }

    pub fn address_generator_failure(reason: impl Into<String>) -> Self {
        Self::AddressGeneratorFailure(reason.into())
    }

    pub fn duplicated_contract_address(address: impl Into<String>) -> Self {
        Self::DuplicatedContractAddress(address.into())
    }
}
