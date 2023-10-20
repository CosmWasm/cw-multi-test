use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{Addr, Binary, Empty};
use cw_multi_test::{AcceptingModule, App, AppResponse, Module, StargateMsg, StargateQuery};

/// Utility function for comparing responses.
fn eq(actual: AppResponse, expected: AppResponse) {
    assert_eq!(actual.events, expected.events);
    assert_eq!(actual.data, expected.data);
}

/// Utility function for asserting default outputs returned from accepting module.
fn assert_results(accepting_module: AcceptingModule<StargateMsg, StargateQuery, Empty>) {
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
                StargateMsg {
                    type_url: Default::default(),
                    value: Default::default(),
                },
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
                StargateQuery {
                    path: Default::default(),
                    data: Default::default()
                }
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
fn stargate_accepting_module_default_works() {
    assert_results(AcceptingModule::default());
}

#[test]
fn stargate_accepting_module_new_works() {
    assert_results(AcceptingModule::new());
}
