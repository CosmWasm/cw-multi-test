#![cfg(feature = "stargate")]

use crate::test_helpers::stargate;
use crate::{no_init, App, AppBuilder, Executor, StargateAccepting};
use cosmwasm_std::Empty;

#[test]
fn default_failing_stargate_handler_should_work() {
    let mut app = App::default();

    // store the contract
    let creator_addr = app.api().addr_make("creator");
    let code = app.store_code_with_creator(creator_addr, stargate::contract());

    // instantiate contract
    let owner_addr = app.api().addr_make("owner");
    let contract_addr = app
        .instantiate_contract(code, owner_addr.clone(), &Empty {}, &[], "tauri", None)
        .unwrap();

    // execute empty message on the contract, this contract returns stargate message
    // which is rejected by default failing stargate keeper
    let err = app
        .execute_contract(owner_addr, contract_addr, &Empty {}, &[])
        .unwrap_err();

    // source error message comes from failing stargate keeper
    assert!(err
        .source()
        .unwrap()
        .to_string()
        .starts_with("Unexpected stargate execute"));
}

#[test]
fn accepting_stargate_handler_should_work() {
    let mut app = AppBuilder::default()
        .with_stargate(StargateAccepting)
        .build(no_init);

    // store the contract
    let creator_addr = app.api().addr_make("creator");
    let code = app.store_code_with_creator(creator_addr, stargate::contract());

    // instantiate contract
    let owner_addr = app.api().addr_make("owner");
    let contract_addr = app
        .instantiate_contract(code, owner_addr.clone(), &Empty {}, &[], "tauri", None)
        .unwrap();

    // execute empty message on the contract, this contract returns stargate message
    // which is just silently processed by accepting stargate keeper
    assert!(app
        .execute_contract(owner_addr, contract_addr, &Empty {}, &[])
        .is_ok());
}
