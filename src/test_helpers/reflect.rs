use crate::test_helpers::{payout, CustomHelperMsg};
use crate::{Contract, ContractWrapper};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, Event, MessageInfo, Reply, Response,
    StdError, SubMsg,
};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Message {
    pub messages: Vec<SubMsg<CustomHelperMsg>>,
}

#[cw_serde]
pub enum QueryMsg {
    Count {},
    Reply { id: u64 },
}

const COUNT: Item<u32> = Item::new("count");
const REFLECT: Map<u64, Reply> = Map::new("reflect");

fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> Result<Response<CustomHelperMsg>, StdError> {
    COUNT.save(deps.storage, &0)?;
    Ok(Response::default())
}

fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: Message,
) -> Result<Response<CustomHelperMsg>, StdError> {
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

fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response<CustomHelperMsg>, StdError> {
    REFLECT.save(deps.storage, msg.id, &msg)?;
    // add custom event here to test
    let event = Event::new("custom")
        .add_attribute("from", "reply")
        .add_attribute("to", "test");
    Ok(Response::new().add_event(event))
}

pub fn contract() -> Box<dyn Contract<CustomHelperMsg>> {
    let contract = ContractWrapper::new(execute, instantiate, query).with_reply(reply);
    Box::new(contract)
}
