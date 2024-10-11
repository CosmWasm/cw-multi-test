//! # Echo contract
//!
//! Simple echoing contract which just returns incoming data.

use crate::{Contract, ContractWrapper};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Attribute, Binary, CustomMsg, Deps, DepsMut, Empty, Env, Event, MessageInfo,
    Reply, Response, StdError, StdResult, SubMsg, SubMsgResponse, SubMsgResult,
};
use cw_utils::{parse_execute_response_data, parse_instantiate_response_data};
use serde::de::DeserializeOwned;

/// Base identifier value for message replies.
///
/// By convention, choosing a reply identifier less than EXECUTE_REPLY_BASE_ID indicates
/// an `Instantiate` message reply, otherwise it indicates the `Execute` message reply.
pub const EXECUTE_REPLY_BASE_ID: u64 = i64::MAX as u64;

#[cw_serde]
#[derive(Default)]
pub struct InitMessage<C>
where
    C: CustomMsg + 'static,
{
    pub data: Option<String>,
    pub sub_msg: Option<Vec<SubMsg<C>>>,
}

#[cw_serde]
#[derive(Default)]
pub struct ExecMessage<C>
where
    C: CustomMsg + 'static,
{
    pub data: Option<String>,
    pub sub_msg: Vec<SubMsg<C>>,
    pub attributes: Vec<Attribute>,
    pub events: Vec<Event>,
}

fn instantiate<C>(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMessage<C>,
) -> StdResult<Response<C>>
where
    C: CustomMsg + 'static,
{
    let mut response = Response::new();
    if let Some(data) = msg.data {
        response = response.set_data(data.into_bytes());
    }
    if let Some(msgs) = msg.sub_msg {
        response = response.add_submessages(msgs);
    }
    Ok(response)
}

fn execute<C>(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecMessage<C>,
) -> StdResult<Response<C>>
where
    C: CustomMsg + 'static,
{
    let mut response = Response::new();
    if let Some(data) = msg.data {
        response = response.set_data(data.into_bytes());
    }
    response = response
        .add_submessages(msg.sub_msg)
        .add_attributes(msg.attributes)
        .add_events(msg.events);
    Ok(response)
}

fn query(_deps: Deps, _env: Env, msg: Empty) -> StdResult<Binary> {
    to_json_binary(&msg)
}

fn reply<C>(_deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response<C>>
where
    C: CustomMsg + 'static,
{
    let response = Response::new();
    #[allow(deprecated)]
    if let Reply {
        id,
        result: SubMsgResult::Ok(SubMsgResponse {
            data: Some(data), ..
        }),
        ..
    } = msg
    {
        let parsed_data = if id < EXECUTE_REPLY_BASE_ID {
            // parse out the WasmMsg::Execute wrapper for instantiate reply
            parse_instantiate_response_data(data.as_slice())
                .map_err(|e| StdError::generic_err(e.to_string()))?
                .data
        } else {
            // parse out the WasmMsg::Execute wrapper for execute reply
            parse_execute_response_data(data.as_slice())
                .map_err(|e| StdError::generic_err(e.to_string()))?
                .data
        };
        if let Some(data) = parsed_data {
            Ok(response.set_data(data))
        } else {
            Ok(response)
        }
    } else {
        Ok(response)
    }
}

pub fn contract<C>() -> Box<dyn Contract<C>>
where
    C: CustomMsg + DeserializeOwned + 'static,
{
    Box::new(ContractWrapper::new(execute::<C>, instantiate::<C>, query).with_reply(reply::<C>))
}
