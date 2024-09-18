//! Very simple echoing contract which just returns incoming string if any,
//! but performing sub call of given message to test response.
//!
//! Additionally, it bypasses all events and attributes send to it.

use crate::{Contract, ContractWrapper};
use cosmwasm_std::{
    to_json_binary, Attribute, Binary, CustomMsg, Deps, DepsMut, Empty, Env, Event, MessageInfo,
    Reply, Response, StdError, StdResult, SubMsg, SubMsgResponse, SubMsgResult,
};

use cosmwasm_schema::cw_serde;
use cw_utils::{parse_execute_response_data, parse_instantiate_response_data};
use serde::de::DeserializeOwned;

// Choosing a reply id less than ECHO_EXECUTE_BASE_ID indicates an Instantiate message reply by convention.
// An Execute message reply otherwise.
pub const EXECUTE_REPLY_BASE_ID: u64 = i64::MAX as u64;

#[cw_serde]
#[derive(Default)]
pub struct InitMessage<ExecC>
where
    ExecC: CustomMsg + 'static,
{
    pub data: Option<String>,
    pub sub_msg: Option<Vec<SubMsg<ExecC>>>,
}

#[cw_serde]
#[derive(Default)]
pub struct Message<ExecC>
where
    ExecC: CustomMsg + 'static,
{
    pub data: Option<String>,
    pub sub_msg: Vec<SubMsg<ExecC>>,
    pub attributes: Vec<Attribute>,
    pub events: Vec<Event>,
}

fn instantiate<ExecC>(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMessage<ExecC>,
) -> StdResult<Response<ExecC>>
where
    ExecC: CustomMsg + 'static,
{
    let mut res = Response::new();
    if let Some(data) = msg.data {
        res = res.set_data(data.into_bytes());
    }
    if let Some(msgs) = msg.sub_msg {
        res = res.add_submessages(msgs);
    }
    Ok(res)
}

fn execute<ExecC>(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: Message<ExecC>,
) -> StdResult<Response<ExecC>>
where
    ExecC: CustomMsg + 'static,
{
    let mut res = Response::new();
    if let Some(data) = msg.data {
        res = res.set_data(data.into_bytes());
    }
    Ok(res
        .add_submessages(msg.sub_msg)
        .add_attributes(msg.attributes)
        .add_events(msg.events))
}

fn query(_deps: Deps, _env: Env, msg: Empty) -> StdResult<Binary> {
    to_json_binary(&msg)
}

fn reply<ExecC>(_deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response<ExecC>>
where
    ExecC: CustomMsg + 'static,
{
    #[allow(deprecated)]
    if let Reply {
        id,
        result:
            SubMsgResult::Ok(SubMsgResponse {
                events: _,
                data: Some(data),
                msg_responses: _,
            }),
        ..
    } = msg
    {
        // We parse out the WasmMsg::Execute wrapper...
        // TODO: Handle all of Execute, Instantiate, and BankMsg replies differently.
        let parsed_data = if id < EXECUTE_REPLY_BASE_ID {
            parse_instantiate_response_data(data.as_slice())
                .map_err(|e| StdError::generic_err(e.to_string()))?
                .data
        } else {
            parse_execute_response_data(data.as_slice())
                .map_err(|e| StdError::generic_err(e.to_string()))?
                .data
        };

        let res = Response::default();
        if let Some(data) = parsed_data {
            Ok(res.set_data(data))
        } else {
            Ok(res)
        }
    } else {
        Ok(Response::default())
    }
}

pub fn contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(execute::<Empty>, instantiate::<Empty>, query)
            .with_reply(reply::<Empty>),
    )
}

pub fn custom_contract<C>() -> Box<dyn Contract<C>>
where
    C: CustomMsg + DeserializeOwned + 'static,
{
    Box::new(ContractWrapper::new(execute::<C>, instantiate::<C>, query).with_reply(reply::<C>))
}
