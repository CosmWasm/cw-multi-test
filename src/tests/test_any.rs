use crate::test_helpers::any;
use crate::{no_init, App, AppBuilder, Executor, StargateAcceptingModule};
use cosmwasm_std::Empty;

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
    // which is rejected by default failing stargate keeper
    let err = app
        .execute_contract(owner_addr, contract_addr, &Empty {}, &[])
        .unwrap_err();

    // source error message comes from failing stargate keeper
    assert!(err
        .source()
        .unwrap()
        .to_string()
        .starts_with("Unexpected exec msg AnyMsg"));
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
    // which is just silently processed by accepting stargate keeper
    assert!(app
        .execute_contract(owner_addr, contract_addr, &Empty {}, &[])
        .is_ok());
}
