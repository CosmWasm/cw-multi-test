use crate::{Contract, ContractWrapper};
use cosmwasm_std::{
    Binary, CustomMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError, SubMsg, WasmMsg,
};

fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
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

fn query(_deps: Deps, _env: Env, _msg: Empty) -> Result<Binary, StdError> {
    Err(StdError::msg(
        "query not implemented for the `caller` contract",
    ))
}

pub fn contract<C>() -> Box<dyn Contract<C>>
where
    C: CustomMsg + 'static,
{
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}
