#![cfg(feature = "cosmwasm_1_2")]

use crate::test_addresses::MockAddressGenerator;
use crate::test_api::MockApiBech32;
use crate::test_contracts::counter;
use cosmwasm_std::{to_json_binary, Empty, WasmMsg};
use cw_multi_test::{AppBuilder, Executor, WasmKeeper};
use cw_utils::parse_instantiate_response_data;

#[test]
fn instantiate2_works() {
    // prepare the application with custom Api and custom address generator
    let mut app = AppBuilder::default()
        .with_api(MockApiBech32::new("juno"))
        .with_wasm(WasmKeeper::default().with_address_generator(MockAddressGenerator))
        .build(|_, _, _| {});

    // prepare addresses for sender and creator
    let sender = app.api().addr_make("sender");
    let creator = app.api().addr_make("creator");

    // store the contract's code
    let code_id = app.store_code_with_creator(creator, counter::contract());

    // prepare tha salt for predictable address
    let salt = "bad kids".as_bytes();

    // instantiate the contract with predictable address
    let init_msg = to_json_binary(&Empty {}).unwrap();
    let msg = WasmMsg::Instantiate2 {
        admin: None,
        code_id,
        msg: init_msg,
        funds: vec![],
        label: "label".into(),
        salt: salt.into(),
    };
    let res = app.execute(sender, msg.into()).unwrap();

    // check the instantiate result
    let parsed = parse_instantiate_response_data(res.data.unwrap().as_slice()).unwrap();
    assert!(parsed.data.is_none());

    // check the predictable contract's address
    assert_eq!(
        parsed.contract_address,
        "juno1navvz5rjlvn43xjqxlpl7dunk6hglmhuh7c6a53eq6qamfam3dus7a220h"
    );
}
