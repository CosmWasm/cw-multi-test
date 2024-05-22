#![cfg(feature = "cosmwasm_1_2")]

use crate::test_contracts::counter;
use cosmwasm_std::testing::MockApi;
use cw_multi_test::App;

#[test]
fn storing_code_assigns_consecutive_identifiers() {
    // prepare the application
    let mut app = App::default();

    // storing contract's code assigns consecutive code identifiers
    for i in 1..=10 {
        assert_eq!(i, app.store_code(counter::contract()));
    }
}

#[test]
fn store_code_generates_default_address_for_creator() {
    // prepare the application
    let mut app = App::default();

    // store contract's code
    let code_id = app.store_code(counter::contract());
    assert_eq!(1, code_id);

    // retrieve contract code info
    let code_info_response = app.wrap().query_wasm_code_info(code_id).unwrap();

    // address of the creator should be the default one
    assert_eq!(
        MockApi::default().addr_make("creator").as_str(),
        code_info_response.creator.as_str()
    );
}
