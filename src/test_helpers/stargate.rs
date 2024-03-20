use crate::{Contract, ContractWrapper};
use cosmwasm_std::{
    AnyMsg, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};

fn instantiate(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

fn execute(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new().add_message(CosmosMsg::Any(AnyMsg {
        type_url: "/this.is.a.stargate.test.helper".to_string(),
        value: Default::default(),
    })))
}

fn query(_deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
    Ok(Binary::default())
}

pub fn contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}
