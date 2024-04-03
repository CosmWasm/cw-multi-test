use crate::test_helpers::stargate;
use crate::{no_init, App, AppBuilder, Executor, StargateAcceptingModule};
use cosmwasm_std::{Binary, Empty};

#[test]
fn failing_stargate_module_should_work_with_stargate() {
    let mut app = App::default();

    // store the contract
    let creator_addr = app.api().addr_make("creator");
    let code = app.store_code_with_creator(creator_addr, stargate::contract());

    // instantiate contract
    let owner_addr = app.api().addr_make("owner");
    let contract_addr = app
        .instantiate_contract(code, owner_addr.clone(), &Empty {}, &[], "stargate", None)
        .unwrap();

    // execute empty message on the contract, this contract returns 'stargate' message
    // which is rejected by default failing stargate keeper
    // source error message comes from failing stargate keeper 'execute' entry-point
    assert!(app
        .execute_contract(owner_addr, contract_addr.clone(), &Empty {}, &[])
        .unwrap_err()
        .source()
        .unwrap()
        .to_string()
        .starts_with("Unexpected exec msg AnyMsg"));

    // error message comes from failing stargate keeper 'query' entry-point
    assert!(app
        .wrap()
        .query_wasm_smart::<Binary>(contract_addr, &Empty {})
        .unwrap_err()
        .to_string()
        .contains("Unexpected custom query GrpcQuery"));
}

#[test]
fn accepting_stargate_module_should_work_with_stargate() {
    let mut app = AppBuilder::default()
        .with_stargate(StargateAcceptingModule::new())
        .build(no_init);

    // store the contract
    let creator_addr = app.api().addr_make("creator");
    let code = app.store_code_with_creator(creator_addr, stargate::contract());

    // instantiate contract
    let owner_addr = app.api().addr_make("owner");
    let contract_addr = app
        .instantiate_contract(code, owner_addr.clone(), &Empty {}, &[], "stargate", None)
        .unwrap();

    // execute empty message on the contract, this contract returns 'stargate' message
    // which is just silently processed by accepting stargate keeper
    assert!(app
        .execute_contract(owner_addr, contract_addr.clone(), &Empty {}, &[])
        .is_ok());

    // query with empty message, which is just silently processed by accepting stargate keeper
    assert!(app
        .wrap()
        .query_wasm_smart::<Empty>(contract_addr, &Empty {})
        .is_ok());
}
