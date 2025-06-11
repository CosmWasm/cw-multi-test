#[test]
fn sudo_entrypoint_should_work() {
    use cosmwasm_std::{from_json, Empty};
    use cw_multi_test::{App, Contract, ContractWrapper, Executor, IntoAddr};

    // Contract definition.
    mod the_contract {
        use cosmwasm_schema::cw_serde;
        use cosmwasm_std::{
            to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
        };

        #[cw_serde]
        pub struct SudoMsg {
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

        pub fn sudo(_deps: DepsMut, _env: Env, msg: SudoMsg) -> StdResult<Response> {
            Ok(Response::default().set_data(to_json_binary(&msg)?))
        }
    }

    use the_contract::*;

    // Returns the wrapped contract.
    pub fn contract() -> Box<dyn Contract<Empty>> {
        Box::new(ContractWrapper::new(execute, instantiate, query).with_sudo(sudo))
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

    // Calling `sudo` entrypoint should work.
    let msg = SudoMsg {
        some_string: "some string".to_string(),
    };
    let app_response = app.wasm_sudo(contract_addr, &msg).unwrap();
    let response_msg: SudoMsg = from_json(app_response.data.unwrap()).unwrap();
    assert_eq!("some string", response_msg.some_string);
}

#[test]
fn sudo_empty_entrypoint_should_work() {
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

        pub fn sudo(_deps: DepsMut, _env: Env, _msg: Empty) -> StdResult<Response> {
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
            .with_sudo_empty(the_contract::sudo),
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

    // Calling `sudo` entrypoint should work.
    let res = app.wasm_sudo(contract_addr, &Empty {}).unwrap();
    assert_eq!(None, res.data);
}
