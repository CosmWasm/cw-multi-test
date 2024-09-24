use crate::test_helpers::COUNT;
use crate::{Contract, ContractWrapper};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, BankMsg, Binary, Coin, CustomMsg, Deps, DepsMut, Empty, Env, MessageInfo,
    Response, StdError,
};
use cw_storage_plus::Item;

#[cw_serde]
pub struct InstantiateMessage {
    pub payout: Coin,
}

#[cw_serde]
pub struct SudoMsg {
    pub set_count: u32,
}

#[cw_serde]
pub enum QueryMsg {
    Count {},
    Payout {},
}

#[cw_serde]
pub struct CountResponse {
    pub count: u32,
}

const PAYOUT: Item<InstantiateMessage> = Item::new("payout");

fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMessage,
) -> Result<Response, StdError> {
    PAYOUT.save(deps.storage, &msg)?;
    COUNT.save(deps.storage, &1)?;
    Ok(Response::default())
}

fn execute(deps: DepsMut, _env: Env, info: MessageInfo, _msg: Empty) -> Result<Response, StdError> {
    // always try to payout what was set originally
    let payout = PAYOUT.load(deps.storage)?;
    let msg = BankMsg::Send {
        to_address: info.sender.into(),
        amount: vec![payout.payout],
    };
    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "payout"))
}

fn sudo(deps: DepsMut, _env: Env, msg: SudoMsg) -> Result<Response, StdError> {
    COUNT.save(deps.storage, &msg.set_count)?;
    Ok(Response::default())
}

fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, StdError> {
    match msg {
        QueryMsg::Count {} => {
            let count = COUNT.load(deps.storage)?;
            let res = CountResponse { count };
            to_json_binary(&res)
        }
        QueryMsg::Payout {} => {
            let payout = PAYOUT.load(deps.storage)?;
            to_json_binary(&payout)
        }
    }
}

pub fn contract<C>() -> Box<dyn Contract<C>>
where
    C: CustomMsg + 'static,
{
    let contract =
        ContractWrapper::new_with_empty(execute, instantiate, query).with_sudo_empty(sudo);
    Box::new(contract)
}
