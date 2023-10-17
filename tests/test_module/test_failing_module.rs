use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{App, FailingModule, Module};

#[test]
fn failing_module_default_works() {
    let failing_module: FailingModule<Empty, Empty, Empty> = FailingModule::default();
    let app = App::default();
    let mut storage = MockStorage::default();
    assert_eq!(
        r#"Unexpected exec msg Empty from Addr("sender")"#,
        failing_module
            .execute(
                app.api(),
                &mut storage,
                app.router(),
                &app.block_info(),
                Addr::unchecked("sender"),
                Empty {}
            )
            .unwrap_err()
            .to_string()
    );
    assert_eq!(
        "Unexpected custom query Empty",
        failing_module
            .query(
                app.api(),
                &storage,
                &(*app.wrap()),
                &app.block_info(),
                Empty {}
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
fn failing_module_new_works() {
    let failing_module: FailingModule<Empty, Empty, Empty> = FailingModule::new();
    let app = App::default();
    let mut storage = MockStorage::default();
    assert_eq!(
        r#"Unexpected exec msg Empty from Addr("sender")"#,
        failing_module
            .execute(
                app.api(),
                &mut storage,
                app.router(),
                &app.block_info(),
                Addr::unchecked("sender"),
                Empty {}
            )
            .unwrap_err()
            .to_string()
    );
    assert_eq!(
        "Unexpected custom query Empty",
        failing_module
            .query(
                app.api(),
                &storage,
                &(*app.wrap()),
                &app.block_info(),
                Empty {}
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
