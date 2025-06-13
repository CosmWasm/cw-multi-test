//! Simplified contract which when executed releases the funds to beneficiary

use crate::{Contract, ContractWrapper};
use cosmwasm_schema::cw_serde;
#[cfg(feature = "cosmwasm_2_2")]
use cosmwasm_std::MigrateInfo;
use cosmwasm_std::{
    to_json_binary, BankMsg, Binary, CustomMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StdError,
};
use cw_storage_plus::Item;

#[cw_serde]
pub struct InstantiateMsg {
    pub beneficiary: String,
}

#[cw_serde]
pub struct MigrateMsg {
    // just use some other string, so we see there are other types
    pub new_guy: String,
}

#[cw_serde]
pub enum QueryMsg {
    // returns InstantiateMsg
    Beneficiary {},
}

const HACKATOM: Item<InstantiateMsg> = Item::new("hackatom");

fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, StdError> {
    HACKATOM.save(deps.storage, &msg)?;
    Ok(Response::default())
}

fn execute(deps: DepsMut, env: Env, _info: MessageInfo, _msg: Empty) -> Result<Response, StdError> {
    let init = HACKATOM.load(deps.storage)?;
    #[allow(deprecated)]
    let balance = deps.querier.query_balance(env.contract.address, "btc")?;

    let resp = Response::new().add_message(BankMsg::Send {
        to_address: init.beneficiary,
        amount: vec![balance],
    });

    Ok(resp)
}

fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, StdError> {
    match msg {
        QueryMsg::Beneficiary {} => {
            let res = HACKATOM.load(deps.storage)?;
            to_json_binary(&res)
        }
    }
}

#[cfg(not(feature = "cosmwasm_2_2"))]
fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, StdError> {
    HACKATOM.update::<_, StdError>(deps.storage, |mut state| {
        state.beneficiary = msg.new_guy;
        Ok(state)
    })?;
    let resp = Response::new().add_attribute("migrate", "successful");
    Ok(resp)
}

#[cfg(feature = "cosmwasm_2_2")]
fn migrate(
    deps: DepsMut,
    _env: Env,
    msg: MigrateMsg,
    _info: MigrateInfo,
) -> Result<Response, StdError> {
    HACKATOM.update::<_, StdError>(deps.storage, |mut state| {
        state.beneficiary = msg.new_guy;
        Ok(state)
    })?;
    let resp = Response::new().add_attribute("migrate", "successful");
    Ok(resp)
}

pub fn contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query).with_migrate(migrate);
    Box::new(contract)
}

#[allow(dead_code)]
pub fn custom_contract<C>() -> Box<dyn Contract<C>>
where
    C: CustomMsg + 'static,
{
    let contract =
        ContractWrapper::new_with_empty(execute, instantiate, query).with_migrate_empty(migrate);
    Box::new(contract)
}
