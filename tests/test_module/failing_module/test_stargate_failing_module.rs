use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{App, FailingModule, Module, StargateMsg, StargateQuery};

/// Utility function for asserting outputs returned from failing module.
fn assert_results(failing_module: FailingModule<StargateMsg, StargateQuery, Empty>) {
    let app = App::default();
    let mut storage = MockStorage::default();
    assert_eq!(
        r#"Unexpected exec msg StargateMsg { type_url: "", value: Binary() } from Addr("sender")"#,
        failing_module
            .execute(
                app.api(),
                &mut storage,
                app.router(),
                &app.block_info(),
                Addr::unchecked("sender"),
                StargateMsg {
                    type_url: Default::default(),
                    value: Default::default(),
                }
            )
            .unwrap_err()
            .to_string()
    );
    assert_eq!(
        r#"Unexpected custom query StargateQuery { path: "", data: Binary() }"#,
        failing_module
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
            .unwrap_err()
            .to_string()
    );
    assert_eq!(
        "Unexpected sudo msg Empty",
        failing_module
            .sudo(
                app.api(),
                &mut storage,
                app.router(),
                &app.block_info(),
                Empty {}
            )
            .unwrap_err()
            .to_string()
    );
}

#[test]
fn stargate_failing_module_default_works() {
    assert_results(FailingModule::default());
}

#[test]
fn stargate_failing_module_new_works() {
    assert_results(FailingModule::new());
}
