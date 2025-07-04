//! # Error definitions

use cosmwasm_std::{WasmMsg, WasmQuery};

macro_rules! std_error_bail {
    ($msg:literal $(,)?) => {
        return Err(StdError::msg($msg))
    };
    ($err:expr $(,)?) => {
        return Err(StdError::msg($err))
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err(StdError::msg(format!($fmt, $($arg)*)))
    };
}

pub(crate) use std_error_bail;

macro_rules! std_error {
    ($msg:literal $(,)?) => {
        StdError::msg($msg)
    };
    ($err:expr $(,)?) => {
        StdError::msg($err)
    };
    ($fmt:expr, $($arg:tt)*) => {
        StdError::msg(format!($fmt, $($arg)*))
    };
}

pub(crate) use std_error;

/// Creates an instance of the error for empty attribute key.
pub fn empty_attribute_key(value: impl Into<String>) -> String {
    format!("Empty attribute key. Value: {0}", value.into())
}

/// Creates an instance of the error when reserved attribute key was used.
pub fn reserved_attribute_key(key: impl Into<String>) -> String {
    format!(
        "Attribute key starts with reserved prefix _: {0}",
        key.into()
    )
}

/// Creates an instance of the error for too short event types.
pub fn event_type_too_short(ty: impl Into<String>) -> String {
    format!("Event type too short: {0}", ty.into())
}

/// Creates an instance of the error for unsupported wasm queries.
pub fn unsupported_wasm_query(query: WasmQuery) -> String {
    format!("Unsupported wasm query: {query:?}")
}

/// Creates an instance of the error for unsupported wasm messages.
pub fn unsupported_wasm_message(msg: WasmMsg) -> String {
    format!("Unsupported wasm message: {msg:?}")
}

/// Creates an instance of the error for invalid contract code identifier.
pub fn invalid_code_id() -> String {
    "code id: invalid".to_string()
}

/// Creates an instance of the error for unregistered contract code identifier.
pub fn unregistered_code_id(code_id: u64) -> String {
    format!("code id {code_id}: no such code")
}

/// Creates an instance of the error for duplicated contract code identifier.
pub fn duplicated_code_id(code_id: u64) -> String {
    format!("duplicated code id {code_id}")
}

/// Creates an instance of the error for exhausted contract code identifiers.
pub fn no_more_code_id_available() -> String {
    "no more code identifiers available".to_string()
}

/// Creates an instance of the error for duplicated contract addresses.
pub fn duplicated_contract_address(addr: impl Into<String>) -> String {
    format!(
        "Contract with this address already exists: {0}",
        addr.into()
    )
}
