use crate::custom_handler::CachingCustomHandler;
use crate::test_helpers::CustomMsg;
use crate::{App, Module};
use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{Addr, Empty};

#[test]
fn custom_handler_works() {
    // prepare needed tools
    let app = App::default();
    let mut storage = MockStorage::default();

    // create custom handler
    let custom_handler = CachingCustomHandler::<CustomMsg, CustomMsg>::new();

    // run execute function
    let _ = custom_handler.execute(
        app.api(),
        &mut storage,
        app.router(),
        &app.block_info(),
        Addr::unchecked("sender"),
        CustomMsg::SetAge { age: 32 },
    );

    // run query function
    let _ = custom_handler.query(
        app.api(),
        &mut storage,
        &(*app.wrap()),
        &app.block_info(),
        CustomMsg::SetName {
            name: "John".to_string(),
        },
    );

    let custom_handler_state = custom_handler.state();

    assert_eq!(
        custom_handler_state.execs().to_owned(),
        vec![CustomMsg::SetAge { age: 32 }]
    );

    assert_eq!(
        custom_handler_state.queries().to_owned(),
        vec![CustomMsg::SetName {
            name: "John".to_string()
        }]
    );

    custom_handler_state.reset();
    assert!(custom_handler_state.execs().is_empty());
    assert!(custom_handler_state.queries().is_empty());
}

#[test]
fn custom_handler_has_no_sudo() {
    // prepare needed tools
    let app = App::default();
    let mut storage = MockStorage::default();

    // create custom handler
    let custom_handler = CachingCustomHandler::<CustomMsg, CustomMsg>::new();

    // run sudo function
    assert_eq!(
        "Unexpected sudo msg Empty",
        custom_handler
            .sudo(
                app.api(),
                &mut storage,
                app.router(),
                &app.block_info(),
                Empty {},
            )
            .unwrap_err()
            .to_string()
    );
}
