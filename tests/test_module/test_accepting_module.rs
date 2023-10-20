use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{Addr, Binary, Empty};
use cw_multi_test::{AcceptingModule, App, AppResponse, Module};

/// Utility function for comparing responses.
fn eq(actual: AppResponse, expected: AppResponse) {
    assert_eq!(actual.events, expected.events);
    assert_eq!(actual.data, expected.data);
}

#[test]
fn accepting_module_default_works() {
    let accepting_module: AcceptingModule<Empty, Empty, Empty> = AcceptingModule::default();
    let app = App::default();
    let mut storage = MockStorage::default();
    eq(
        AppResponse::default(),
        accepting_module
            .execute(
                app.api(),
                &mut storage,
                app.router(),
                &app.block_info(),
                Addr::unchecked("sender"),
                Empty {},
            )
            .unwrap(),
    );
    assert_eq!(
        Binary::default(),
        accepting_module
            .query(
                app.api(),
                &storage,
                &(*app.wrap()),
                &app.block_info(),
                Empty {}
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
                Empty {},
            )
            .unwrap(),
    );
}

#[test]
fn accepting_module_new_works() {
    let accepting_module: AcceptingModule<Empty, Empty, Empty> = AcceptingModule::new();
    let app = App::default();
    let mut storage = MockStorage::default();
    eq(
        AppResponse::default(),
        accepting_module
            .execute(
                app.api(),
                &mut storage,
                app.router(),
                &app.block_info(),
                Addr::unchecked("sender"),
                Empty {},
            )
            .unwrap(),
    );
    assert_eq!(
        Binary::default(),
        accepting_module
            .query(
                app.api(),
                &storage,
                &(*app.wrap()),
                &app.block_info(),
                Empty {}
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
                Empty {},
            )
            .unwrap(),
    );
}
