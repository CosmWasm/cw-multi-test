use crate::custom_handler::CachingCustomHandler;
use crate::test_helpers::CustomHelperMsg;
use crate::{App, Module};
use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::Empty;

///Custom handlers in CosmWasm allow developers to incorporate their own unique logic into tests.
///This feature is valuable for tailoring the testing environment to reflect specific
/// use-cases or behaviors in a CosmWasm-based smart contract.
#[test]
fn custom_handler_works() {
    // prepare needed tools
    let app = App::default();
    let mut storage = MockStorage::default();

    // create custom handler
    let custom_handler = CachingCustomHandler::<CustomHelperMsg, CustomHelperMsg>::new();

    // run execute function
    let _ = custom_handler.execute(
        app.api(),
        &mut storage,
        app.router(),
        &app.block_info(),
        app.api().addr_make("sender"),
        CustomHelperMsg::SetAge { age: 32 },
    );

    // run query function
    let _ = custom_handler.query(
        app.api(),
        &storage,
        &(*app.wrap()),
        &app.block_info(),
        CustomHelperMsg::SetName {
            name: "John".to_string(),
        },
    );

    // get the state
    let custom_handler_state = custom_handler.state();

    // there should be one exec message
    assert_eq!(
        custom_handler_state.execs().to_owned(),
        vec![CustomHelperMsg::SetAge { age: 32 }]
    );

    // there should be one query message
    assert_eq!(
        custom_handler_state.queries().to_owned(),
        vec![CustomHelperMsg::SetName {
            name: "John".to_string()
        }]
    );

    // clear the state and assert there are no more messages
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
    let custom_handler = CachingCustomHandler::<CustomHelperMsg, CustomHelperMsg>::new();

    // run sudo function
    assert_eq!(
        "Unexpected custom sudo message Empty",
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
