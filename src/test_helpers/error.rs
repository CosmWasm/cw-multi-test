use crate::{Contract, ContractWrapper};
use cosmwasm_std::{Binary, CustomMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError};

fn instantiate_err(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> Result<Response, StdError> {
    Err(StdError::msg("Init failed"))
}

fn instantiate_ok(
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
    _msg: Empty,
) -> Result<Response, StdError> {
    Err(StdError::msg("Handle failed"))
}

fn query(_deps: Deps, _env: Env, _msg: Empty) -> Result<Binary, StdError> {
    Err(StdError::msg("Query failed"))
}

pub fn contract<C>(instantiable: bool) -> Box<dyn Contract<C>>
where
    C: CustomMsg + 'static,
{
    let contract = if instantiable {
        ContractWrapper::new_with_empty(execute, instantiate_ok, query)
    } else {
        ContractWrapper::new_with_empty(execute, instantiate_err, query)
    };
    Box::new(contract)
}
