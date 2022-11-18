use cosmwasm_std::{
    Binary, CosmosMsg, Deps, DepsMut, Empty, Env, GovMsg, IbcMsg, MessageInfo, Response, StdResult,
};
use serde::{Deserialize, Serialize};

use crate::{Contract, ContractWrapper};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecMsg {
    Ibc {},
    Gov {},
}

fn instantiate(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}

fn execute(_deps: DepsMut, _env: Env, _info: MessageInfo, msg: ExecMsg) -> StdResult<Response> {
    let msg: CosmosMsg = if let ExecMsg::Ibc {} = msg {
        IbcMsg::CloseChannel {
            channel_id: "channel".to_string(),
        }
        .into()
    } else {
        GovMsg::Vote {
            proposal_id: 1,
            vote: cosmwasm_std::VoteOption::No,
        }
        .into()
    };

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
