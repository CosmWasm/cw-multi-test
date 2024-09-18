//! # Reflecting contract

use crate::test_helpers::{payout, CustomHelperMsg};
use crate::{Contract, ContractWrapper};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, Event, MessageInfo, Reply, Response,
    StdError, StdResult, SubMsg,
};
use cw_storage_plus::{Item, Map};

#[cw_serde]
#[derive(Default)]
pub struct ExecMessage {
    pub sub_msg: Vec<SubMsg<CustomHelperMsg>>,
}

#[cw_serde]
pub enum QueryMsg {
    Count {},
    Reply { id: u64 },
}

const COUNTER: Item<u32> = Item::new("counter");
const REFLECT: Map<u64, Reply> = Map::new("reflect");

fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response<CustomHelperMsg>> {
    COUNTER.save(deps.storage, &0)?;
    Ok(Response::default())
}

fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecMessage,
) -> StdResult<Response<CustomHelperMsg>> {
    COUNTER.update::<_, StdError>(deps.storage, |value| Ok(value + 1))?;
    Ok(Response::new().add_submessages(msg.sub_msg))
}

fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Count {} => {
            let count = COUNTER.load(deps.storage)?;
            let res = payout::CountResponse { count };
            to_json_binary(&res)
        }
        QueryMsg::Reply { id } => {
            let reply = REFLECT.load(deps.storage, id)?;
            to_json_binary(&reply)
        }
    }
}

fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response<CustomHelperMsg>> {
    REFLECT.save(deps.storage, msg.id, &msg)?;
    // add custom event here to test
    let event = Event::new("custom")
        .add_attribute("from", "reply")
        .add_attribute("to", "test");
    Ok(Response::new().add_event(event))
}

pub fn contract() -> Box<dyn Contract<CustomHelperMsg>> {
    Box::new(ContractWrapper::new(execute, instantiate, query).with_reply(reply))
}
