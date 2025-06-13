#[test]
fn migrate_entrypoint_should_work() {
    use cosmwasm_std::{from_json, Empty};
    use cw_multi_test::{App, Contract, ContractWrapper, Executor, IntoAddr};
    use cw_utils::parse_execute_response_data;

    // Contract definition.
    mod the_contract {
        use cosmwasm_schema::cw_serde;
        use cosmwasm_std::{
            to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
        };

        #[cw_serde]
        pub struct MigrateMsg {
            pub some_string: String,
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

        pub fn query(_deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
            to_json_binary(&Empty {})
        }

        pub fn migrate(_deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
            Ok(Response::default().set_data(to_json_binary(&msg)?))
        }
    }

    use the_contract::*;

    // Returns the wrapped contract.
    pub fn contract() -> Box<dyn Contract<Empty>> {
        Box::new(ContractWrapper::new(execute, instantiate, query).with_migrate(migrate))
    }

    // Initialize the chain.
    let mut app = App::default();

    // Store the contract code on chain.
    let code_id = app.store_code(contract());

    // Prepare addresses.
    let owner_addr = "owner".into_addr();
    let admin_addr = "admin".into_addr();

    // Instantiate the contract.
    let contract_addr = app
        .instantiate_contract(
            code_id,
            owner_addr.clone(),
            &Empty {},
            &[],
            "the-contract",
            Some(admin_addr.to_string()),
        )
        .unwrap();

    // Calling `migrate` entrypoint should work.
    let msg = MigrateMsg {
        some_string: "some string".to_string(),
    };
    let app_response = app
        .migrate_contract(admin_addr, contract_addr, &msg, 1)
        .unwrap();
    let migrate_response = parse_execute_response_data(&app_response.data.unwrap()).unwrap();
    let response_msg: MigrateMsg = from_json(migrate_response.data.unwrap()).unwrap();
    assert_eq!("some string", response_msg.some_string);
}

#[test]
fn migrate_empty_entrypoint_should_work() {
    use cosmwasm_std::Empty;
    use cw_multi_test::{App, Contract, ContractWrapper, Executor, IntoAddr};

    // Contract definition.
    mod the_contract {
        use cosmwasm_std::{
            to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
        };

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

        pub fn query(_deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
            to_json_binary(&Empty {})
        }

        pub fn migrate(_deps: DepsMut, _env: Env, _msg: Empty) -> StdResult<Response> {
            Ok(Response::default())
        }
    }

    // Returns the wrapped contract.
    pub fn contract() -> Box<dyn Contract<Empty>> {
        Box::new(
            ContractWrapper::new(
                the_contract::execute,
                the_contract::instantiate,
                the_contract::query,
            )
            .with_migrate_empty(the_contract::migrate),
        )
    }

    // Initialize the chain.
    let mut app = App::default();

    // Store the contract code on chain.
    let code_id = app.store_code(contract());

    // Prepare addresses.
    let owner_addr = "owner".into_addr();
    let admin_addr = "admin".into_addr();

    // Instantiate the contract.
    let contract_addr = app
        .instantiate_contract(
            code_id,
            owner_addr.clone(),
            &Empty {},
            &[],
            "the-contract",
            Some(admin_addr.to_string()),
        )
        .unwrap();

    // Calling `migrate` entrypoint should work.
    let res = app
        .migrate_contract(admin_addr, contract_addr, &Empty {}, 1)
        .unwrap();
    assert_eq!(None, res.data);
}
