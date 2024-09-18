//! # Reflecting contract

use crate::{Contract, ContractWrapper};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Binary, CustomMsg, Deps, DepsMut, Empty, Env, Event, MessageInfo, Reply,
    Response, StdError, StdResult, SubMsg,
};
use cw_storage_plus::{Item, Map};
use serde::de::DeserializeOwned;

#[cw_serde]
#[derive(Default)]
pub struct ExecMessage<C>
where
    C: CustomMsg + 'static,
{
    pub sub_msg: Vec<SubMsg<C>>,
}

#[cw_serde]
pub enum QueryMsg {
    Count,
    Reply { id: u64 },
}

#[cw_serde]
pub struct ReflectResponse {
    pub count: u32,
}

const COUNTER: Item<u32> = Item::new("counter");
const REFLECT: Map<u64, Reply> = Map::new("reflect");

fn instantiate<C>(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response<C>>
where
    C: CustomMsg + 'static,
{
    COUNTER.save(deps.storage, &0)?;
    Ok(Response::default())
}

fn execute<C>(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecMessage<C>,
) -> StdResult<Response<C>>
where
    C: CustomMsg + 'static,
{
    COUNTER.update::<_, StdError>(deps.storage, |value| Ok(value + 1))?;
    Ok(Response::new().add_submessages(msg.sub_msg))
}

fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Count => {
            let count = COUNTER.load(deps.storage)?;
            to_json_binary(&ReflectResponse { count })
        }
        QueryMsg::Reply { id } => {
            let reply = REFLECT.load(deps.storage, id)?;
            to_json_binary(&reply)
        }
    }
}

fn reply<C>(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response<C>>
where
    C: CustomMsg + 'static,
{
    REFLECT.save(deps.storage, msg.id, &msg)?;
    // add custom event here to test
    let event = Event::new("custom")
        .add_attribute("from", "reply")
        .add_attribute("to", "test");
    Ok(Response::new().add_event(event))
}

pub fn contract<C>() -> Box<dyn Contract<C>>
where
    C: CustomMsg + DeserializeOwned + 'static,
{
    Box::new(ContractWrapper::new(execute::<C>, instantiate::<C>, query).with_reply(reply::<C>))
}
