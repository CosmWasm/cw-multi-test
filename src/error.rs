//! # Error definitions

pub use anyhow::{anyhow, bail, Context as AnyContext, Error as AnyError, Result as AnyResult};
use cosmwasm_std::{WasmMsg, WasmQuery};
use thiserror::Error;

/// An enumeration of errors reported across the **CosmWasm MultiTest** library.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    /// Error variant for reporting an empty attribute key.
    #[error("Empty attribute key. Value: {0}")]
    EmptyAttributeKey(String),

    /// Error variant for reporting an empty attribute value.
    #[error("Empty attribute value. Key: {0}")]
    EmptyAttributeValue(String),

    /// Error variant for reporting a usage of reserved key prefix.
    #[error("Attribute key starts with reserved prefix _: {0}")]
    ReservedAttributeKey(String),

    /// Error variant for reporting too short event types.
    #[error("Event type too short: {0}")]
    EventTypeTooShort(String),

    /// Error variant for reporting that unsupported wasm query was encountered during processing.
    #[error("Unsupported wasm query: {0:?}")]
    UnsupportedWasmQuery(WasmQuery),

    /// Error variant for reporting that unsupported wasm message was encountered during processing.
    #[error("Unsupported wasm message: {0:?}")]
    UnsupportedWasmMsg(WasmMsg),

    /// Error variant for reporting invalid contract code.
    #[error("code id: invalid")]
    InvalidCodeId,

    /// Error variant for reporting unregistered contract code.
    #[error("code id {0}: no such code")]
    UnregisteredCodeId(u64),

    /// Error variant for reporting duplicated contract code identifier.
    #[error("duplicated code id {0}")]
    DuplicatedCodeId(u64),

    /// Error variant for reporting duplicated contract addresses.
    #[error("Contract with this address already exists: {0}")]
    DuplicatedContractAddress(String),
}

impl Error {
    /// Creates an instance of the [Error](Self) for empty attribute key.
    pub fn empty_attribute_key(value: impl Into<String>) -> Self {
        Self::EmptyAttributeKey(value.into())
    }

    /// Creates an instance of the [Error](Self) for empty attribute value.
    pub fn empty_attribute_value(key: impl Into<String>) -> Self {
        Self::EmptyAttributeValue(key.into())
    }

    /// Creates an instance of the [Error](Self) when reserved attribute key was used.
    pub fn reserved_attribute_key(key: impl Into<String>) -> Self {
        Self::ReservedAttributeKey(key.into())
    }

    /// Creates an instance of the [Error](Self) for too short event types.
    pub fn event_type_too_short(ty: impl Into<String>) -> Self {
        Self::EventTypeTooShort(ty.into())
    }

    /// Creates an instance of the [Error](Self) for unsupported wasm queries.
    pub fn unsupported_wasm_query(query: WasmQuery) -> Self {
        Self::UnsupportedWasmQuery(query)
    }

    /// Creates an instance of the [Error](Self) for unsupported wasm messages.
    pub fn unsupported_wasm_message(msg: WasmMsg) -> Self {
        Self::UnsupportedWasmMsg(msg)
    }

    /// Creates an instance of the [Error](Self) for invalid contract code identifier.
    pub fn invalid_contract_code_id() -> Self {
        Self::InvalidCodeId
    }

    /// Creates an instance of the [Error](Self) for unregistered contract code identifier.
    pub fn unregistered_code_id(code_id: u64) -> Self {
        Self::UnregisteredCodeId(code_id)
    }

    /// Creates an instance of the [Error](Self) for duplicated contract code identifier.
    pub fn duplicated_code_id(code_id: u64) -> Self {
        Self::DuplicatedCodeId(code_id)
    }

    /// Creates an instance of the [Error](Self) for duplicated contract addresses.
    pub fn duplicated_contract_address(address: impl Into<String>) -> Self {
        Self::DuplicatedContractAddress(address.into())
    }
}
