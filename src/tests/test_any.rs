use crate::test_helpers::any;
use crate::{no_init, App, AppBuilder, Executor, StargateAcceptingModule};
use cosmwasm_std::{Binary, Empty};

#[test]
fn failing_stargate_module_should_work_with_any() {
    let mut app = App::default();

    // store the contract
    let creator_addr = app.api().addr_make("creator");
    let code = app.store_code_with_creator(creator_addr, any::contract());

    // instantiate contract
    let owner_addr = app.api().addr_make("owner");
    let contract_addr = app
        .instantiate_contract(code, owner_addr.clone(), &Empty {}, &[], "any", None)
        .unwrap();

    // execute empty message on the contract, this contract returns 'any' message
    // which is rejected by always failing keeper (default one)
    // source error message comes from 'execute' entry-point of always failing keeper
    assert!(app
        .execute_contract(owner_addr, contract_addr.clone(), &Empty {}, &[])
        .unwrap_err()
        .source()
        .unwrap()
        .to_string()
        .starts_with("Unexpected exec msg AnyMsg"));

    // error message comes from 'query' entry-point of always failing keeper
    assert!(app
        .wrap()
        .query_wasm_smart::<Binary>(contract_addr, &Empty {})
        .unwrap_err()
        .to_string()
        .contains("Unexpected custom query GrpcQuery"));
}

#[test]
fn accepting_stargate_module_should_work_with_any() {
    let mut app = AppBuilder::default()
        .with_stargate(StargateAcceptingModule::new())
        .build(no_init);

    // store the contract
    let creator_addr = app.api().addr_make("creator");
    let code = app.store_code_with_creator(creator_addr, any::contract());

    // instantiate contract
    let owner_addr = app.api().addr_make("owner");
    let contract_addr = app
        .instantiate_contract(code, owner_addr.clone(), &Empty {}, &[], "any", None)
        .unwrap();

    // execute empty message on the contract, this contract returns 'any' message
    // which is just silently processed by always accepting keeper
    assert!(app
        .execute_contract(owner_addr, contract_addr.clone(), &Empty {}, &[])
        .is_ok());

    // query with empty message, which is just silently processed by always accepting keeper
    assert!(app
        .wrap()
        .query_wasm_smart::<Empty>(contract_addr, &Empty {})
        .is_ok());
}
