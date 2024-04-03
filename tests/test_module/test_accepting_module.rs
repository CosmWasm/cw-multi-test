use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{to_json_binary, Empty};
use cw_multi_test::{AcceptingModule, App, AppResponse, Module};

/// Utility function for comparing responses.
fn eq(actual: AppResponse, expected: AppResponse) {
    assert_eq!(actual.events, expected.events);
    assert_eq!(actual.data, expected.data);
}

/// Utility function for asserting default outputs returned from accepting module.
fn assert_results(accepting_module: AcceptingModule<Empty, Empty, Empty>) {
    let app = App::default();
    let sender_addr = app.api().addr_make("sender");
    let empty_msg = Empty {};
    let mut storage = MockStorage::default();
    eq(
        AppResponse::default(),
        accepting_module
            .execute(
                app.api(),
                &mut storage,
                app.router(),
                &app.block_info(),
                sender_addr,
                empty_msg.clone(),
            )
            .unwrap(),
    );
    assert_eq!(
        to_json_binary(&empty_msg).unwrap(),
        accepting_module
            .query(
                app.api(),
                &storage,
                &(*app.wrap()),
                &app.block_info(),
                empty_msg.clone()
            )
            .unwrap()
    );
    eq(
        AppResponse::default(),
        accepting_module
            .sudo(
                app.api(),
                &mut storage,
                app.router(),
                &app.block_info(),
                empty_msg,
            )
            .unwrap(),
    );
}

#[test]
fn accepting_module_default_works() {
    assert_results(AcceptingModule::default());
}

#[test]
fn accepting_module_new_works() {
    assert_results(AcceptingModule::new());
}
