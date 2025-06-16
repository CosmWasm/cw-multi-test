#![cfg(feature = "cosmwasm_2_2")]

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, MigrateInfo, Response,
    StdResult,
};
use cw_multi_test::{App, Contract, ContractWrapper, Executor, IntoAddr};
use cw_storage_plus::Item;

// The initial version of the contract.
mod contract_one {
    use super::*;

    const VERSION: Item<u32> = Item::new("version");

    #[cw_serde]
    pub struct InstantiateMsg {
        pub value: u32,
    }

    #[cw_serde]
    pub struct ContractResponseMsg {
        pub value: u32,
    }

    pub fn instantiate(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: InstantiateMsg,
    ) -> StdResult<Response> {
        VERSION.save(deps.storage, &msg.value)?;
        Ok(Response::default())
    }

    pub fn execute(
        _deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        _msg: Empty,
    ) -> StdResult<Response> {
        Ok(Response::default())
    }

    pub fn query(deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
        to_json_binary(&ContractResponseMsg {
            value: VERSION.may_load(deps.storage)?.unwrap(),
        })
    }
}

// Contract definition after changes that require migration.
mod contract_two {
    use super::*;

    const NEGATED_VERSION: Item<i64> = Item::new("negated-version");
    const MIGRATION_VERSION: Item<u64> = Item::new("migration-version");

    #[cw_serde]
    pub struct ContractResponseMsg {
        pub negated_version: i64,
        pub migration_version: u64,
    }

    pub fn instantiate(
        _deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        _msg: Empty,
    ) -> StdResult<Response> {
        Ok(Response::default())
    }

    pub fn execute(
        _deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        _msg: Empty,
    ) -> StdResult<Response> {
        Ok(Response::default())
    }

    pub fn query(deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
        to_json_binary(&ContractResponseMsg {
            negated_version: NEGATED_VERSION.may_load(deps.storage)?.unwrap(),
            migration_version: MIGRATION_VERSION.may_load(deps.storage)?.unwrap(),
        })
    }

    pub fn migrate(
        deps: DepsMut,
        _env: Env,
        _msg: Empty,
        info: MigrateInfo,
    ) -> StdResult<Response> {
        const VERSION: Item<u32> = Item::new("version");
        let version = VERSION.may_load(deps.storage)?.unwrap();
        NEGATED_VERSION.save(deps.storage, &-(version as i64))?;
        MIGRATION_VERSION.save(deps.storage, &info.old_migrate_version.unwrap_or(0))?;
        Ok(Response::default())
    }
}

// Returns the wrapped contract before improvements.
pub fn contract_one() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        contract_one::execute,
        contract_one::instantiate,
        contract_one::query,
    ))
}

// Returns the wrapped contract after improvements.
pub fn contract_two() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            contract_two::execute,
            contract_two::instantiate,
            contract_two::query,
        )
        .with_migrate(contract_two::migrate),
    )
}

#[test]
fn migrate_info_should_work() {
    // Initialize the chain.
    let mut app = App::default();

    // Store the code of the contract one on chain.
    let code_id_one = app.store_code(contract_one());

    // Prepare addresses.
    let owner_addr = "owner".into_addr();
    let admin_addr = "admin".into_addr();

    // Instantiate the contract in version one.
    let contract_addr_one = app
        .instantiate_contract(
            code_id_one,
            owner_addr.clone(),
            &contract_one::InstantiateMsg { value: 100 },
            &[],
            "contract-one",
            Some(admin_addr.to_string()),
        )
        .unwrap();

    // Query the state of the contract in version one.
    let response_one: contract_one::ContractResponseMsg = app
        .wrap()
        .query_wasm_smart(contract_addr_one.clone(), &Empty {})
        .unwrap();
    assert_eq!(100, response_one.value);

    // Store the code of the contract two on chain.
    let code_id_two = app.store_code(contract_two());

    // Execute the migration entrypoint.
    // Migrating from contract version one to two and from code id = 1 to code id = 2.
    app.migrate_contract(
        admin_addr.clone(),
        contract_addr_one.clone(),
        &Empty {},
        code_id_two,
    )
    .unwrap();

    // Query the state of the contract in version two.
    let response_two: contract_two::ContractResponseMsg = app
        .wrap()
        .query_wasm_smart(contract_addr_one.clone(), &Empty {})
        .unwrap();
    assert_eq!(-100, response_two.negated_version);
    assert_eq!(1, response_two.migration_version);

    // Store the code of the contract one on chain again.
    let code_id_three = app.store_code(contract_one());

    // Instantiate another contract in version one.
    let contract_addr_two = app
        .instantiate_contract(
            code_id_three,
            owner_addr.clone(),
            &contract_one::InstantiateMsg { value: 200 },
            &[],
            "contract-two",
            Some(admin_addr.to_string()),
        )
        .unwrap();

    // Query the state of the second contract in version one.
    let response_one: contract_one::ContractResponseMsg = app
        .wrap()
        .query_wasm_smart(contract_addr_two.clone(), &Empty {})
        .unwrap();
    assert_eq!(200, response_one.value);

    // Execute the migration entrypoint.
    // Migrating from contract version one to two and from code id = 3 to code id = 2.
    app.migrate_contract(
        admin_addr.clone(),
        contract_addr_two.clone(),
        &Empty {},
        code_id_two,
    )
    .unwrap();

    // Query the state of the contract two in version two.
    let response_two: contract_two::ContractResponseMsg = app
        .wrap()
        .query_wasm_smart(contract_addr_two.clone(), &Empty {})
        .unwrap();
    assert_eq!(-200, response_two.negated_version);
    assert_eq!(3, response_two.migration_version);
}
