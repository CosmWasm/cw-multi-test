#[allow(unused)]
#[test]
fn storing_empty_contract_should_work() {
    use cosmwasm_std::Empty;
    use cw_multi_test::{App, Contract, ContractWrapper};

    // Contract definition.
    mod my_contract {
        use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult};

        pub fn instantiate(
            deps: DepsMut,
            env: Env,
            info: MessageInfo,
            msg: Empty,
        ) -> StdResult<Response> {
            Ok(Response::default())
        }

        pub fn execute(
            deps: DepsMut,
            env: Env,
            info: MessageInfo,
            msg: Empty,
        ) -> StdResult<Response> {
            Ok(Response::default())
        }

        pub fn query(deps: Deps, env: Env, msg: Empty) -> StdResult<Binary> {
            Ok(Binary::default())
        }
    }

    // Wrapped contract.
    pub fn contract() -> Box<dyn Contract<Empty>> {
        Box::new(ContractWrapper::new(
            my_contract::execute,
            my_contract::instantiate,
            my_contract::query,
        ))
    }

    // Chain initialization.
    let mut app = App::default();

    // Storing contract code on chain.
    let code_id = app.store_code(contract());

    assert_eq!(1, code_id);

    // Use this `code_id` to instantiate the contract.
    // ...
}
