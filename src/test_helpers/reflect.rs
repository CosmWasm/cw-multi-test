use crate::test_helpers::{payout, CustomMsg, COUNT};
use crate::{Contract, ContractWrapper};
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, Event, MessageInfo, Reply, Response,
    StdError, SubMsg,
};
use cw_storage_plus::Map;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Message {
    pub messages: Vec<SubMsg<CustomMsg>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryMsg {
    Count {},
    Reply { id: u64 },
}

const REFLECT: Map<u64, Reply> = Map::new("reflect");

fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> Result<Response<CustomMsg>, StdError> {
    COUNT.save(deps.storage, &0)?;
    Ok(Response::default())
}

fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: Message,
) -> Result<Response<CustomMsg>, StdError> {
    COUNT.update::<_, StdError>(deps.storage, |old| Ok(old + 1))?;

    Ok(Response::new().add_submessages(msg.messages))
}

fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, StdError> {
    match msg {
        QueryMsg::Count {} => {
            let count = COUNT.load(deps.storage)?;
            let res = payout::CountResponse { count };
            to_json_binary(&res)
        }
        QueryMsg::Reply { id } => {
            let reply = REFLECT.load(deps.storage, id)?;
            to_json_binary(&reply)
        }
    }
}

fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response<CustomMsg>, StdError> {
    REFLECT.save(deps.storage, msg.id, &msg)?;
    // add custom event here to test
    let event = Event::new("custom")
        .add_attribute("from", "reply")
        .add_attribute("to", "test");
    Ok(Response::new().add_event(event))
}

pub fn contract() -> Box<dyn Contract<CustomMsg>> {
    let contract = ContractWrapper::new(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}
