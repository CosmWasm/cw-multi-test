use crate::test_contracts::counter;
use cw_multi_test::AppBuilder;
use cw_multi_test::{MockApiBech32, MockApiBech32m};

#[test]
fn store_code_with_custom_creator_address_should_work() {
    // prepare the application
    let mut app = AppBuilder::default()
        .with_api(MockApiBech32m::new("juno"))
        .build(|_, _, _| {});

    let creator = app.api().addr_make("zeus");

    // store contract's code
    let code_id = app.store_code_with_creator(creator, counter::contract());
    assert_eq!(1, code_id);

    // retrieve contract code info
    let code_info_response = app.wrap().query_wasm_code_info(code_id).unwrap();

    // address of the creator should be the custom one in Bech32m format
    assert_eq!(
        MockApiBech32m::new("juno").addr_make("zeus"),
        code_info_response.creator
    );

    // address of the creator should be the custom one but not in Bech32 format
    assert_ne!(
        MockApiBech32::new("juno").addr_make("zeus"),
        code_info_response.creator
    );
}
