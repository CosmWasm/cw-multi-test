use crate::{Contract, ContractWrapper};
use cosmwasm_std::{
    AnyMsg, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, GrpcQuery, HexBinary, MessageInfo,
    QueryRequest, Response, StdResult,
};

fn instantiate(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

fn execute(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
    let msg = CosmosMsg::Any(AnyMsg {
        type_url: "/this.is.any.execute.test.helper".to_string(),
        value: Binary::from(HexBinary::from_hex("abc1").unwrap()),
    });
    Ok(Response::new().add_message(msg))
}

fn query(deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
    let request = QueryRequest::Grpc(GrpcQuery {
        path: "/this.is.any.query.test.helper".to_string(),
        data: Binary::from(HexBinary::from_hex("abc2").unwrap()),
    });
    deps.querier.query(&request)
}

pub fn contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}
