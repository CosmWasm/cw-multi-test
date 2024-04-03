use crate::{Contract, ContractWrapper};
use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, HexBinary, MessageInfo,
    QueryRequest, Response, StdResult,
};

fn instantiate(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

fn execute(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
    #[allow(deprecated)]
    let msg = CosmosMsg::Stargate {
        type_url: "/this.is.stargate.execute.test.helper".to_string(),
        value: Binary::from(HexBinary::from_hex("abc1").unwrap()),
    };
    Ok(Response::new().add_message(msg))
}

fn query(deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
    #[allow(deprecated)]
    let request = QueryRequest::Stargate {
        path: "/this.is.stargate.query.test.helper".to_string(),
        data: Binary::from(HexBinary::from_hex("abc2").unwrap()),
    };
    deps.querier
        .query::<Empty>(&request)
        .map(|result| to_json_binary(&result).unwrap())
}

pub fn contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}
