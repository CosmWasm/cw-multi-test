use crate::{test_helpers::EmptyMsg, Contract, ContractWrapper};
use cosmwasm_std::{
    Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, SubMsg, WasmMsg,
};
use schemars::JsonSchema;
use std::fmt;

fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<Response, StdError> {
    Ok(Response::default())
}

fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: WasmMsg,
) -> Result<Response, StdError> {
    let message = SubMsg::new(msg);

    Ok(Response::new().add_submessage(message))
}

fn query(_deps: Deps, _env: Env, _msg: EmptyMsg) -> Result<Binary, StdError> {
    Err(StdError::generic_err(
        "query not implemented for the `caller` contract",
    ))
}

pub fn contract<C>() -> Box<dyn Contract<C>>
where
    C: Clone + fmt::Debug + PartialEq + JsonSchema + 'static,
{
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

/// Returns the address of the creator of this contract.
pub fn creator() -> Addr {
    Addr::unchecked("caller_contract_creator")
}
