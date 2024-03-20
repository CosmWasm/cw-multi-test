use crate::{Contract, ContractWrapper};
use cosmwasm_std::{
    Binary, CosmosMsg, Deps, DepsMut, Empty, Env, GovMsg, MessageInfo, Response, StdResult,
};

fn instantiate(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

fn execute(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
    let msg: CosmosMsg = GovMsg::Vote {
        proposal_id: 1,
        option: cosmwasm_std::VoteOption::No,
    }
    .into();
    let resp = Response::new().add_message(msg);
    Ok(resp)
}

fn query(_deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
    Ok(Binary::default())
}

pub fn contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}
