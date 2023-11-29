#![cfg(feature = "cosmwasm_1_2")]

use crate::test_contracts::counter;
use cosmwasm_std::{instantiate2_address, to_json_binary, Api, Empty, WasmMsg};
use cw_multi_test::addons::{MockAddressGenerator, MockApiBech32};
use cw_multi_test::{no_init, AppBuilder, Executor, WasmKeeper};
use cw_utils::parse_instantiate_response_data;

#[test]
fn instantiate2_works() {
    // prepare the application with custom Api and custom address generator
    let mut app = AppBuilder::default()
        .with_api(MockApiBech32::new("juno"))
        .with_wasm(WasmKeeper::default().with_address_generator(MockAddressGenerator))
        .build(no_init);

    // prepare addresses for sender and creator
    let sender = app.api().addr_make("sender");
    let creator = app.api().addr_make("creator");

    // store the contract's code
    let code_id = app.store_code_with_creator(creator, counter::contract());

    // prepare the salt for predictable address
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
    let res = app.execute(sender.clone(), msg.into()).unwrap();

    // check the instantiate result
    let parsed = parse_instantiate_response_data(res.data.unwrap().as_slice()).unwrap();
    assert!(parsed.data.is_none());

    // check the resulting predictable contract's address
    assert_eq!(
        parsed.contract_address,
        "juno1navvz5rjlvn43xjqxlpl7dunk6hglmhuh7c6a53eq6qamfam3dus7a220h"
    );

    // ----------------------------------------------------------------------
    // Below is an additional check, proving that the predictable address
    // from contract instantiation is exactly the same as the address
    // returned from the function cosmwasm_std::instantiate2_address
    // ----------------------------------------------------------------------

    // get the code info of the contract
    let code_info_response = app.wrap().query_wasm_code_info(code_id).unwrap();

    // retrieve the contract's code checksum
    let checksum = code_info_response.checksum.as_slice();

    // canonicalize the sender address (which is now in human Bech32 format)
    let sender_addr = app.api().addr_canonicalize(sender.as_str()).unwrap();

    // get the contract address using cosmwasm_std::instantiate2_address function
    let contract_addr = instantiate2_address(checksum, &sender_addr, salt).unwrap();

    // humanize the address of the contract
    let contract_human_addr = app.api().addr_humanize(&contract_addr).unwrap();

    // check if the predictable contract's address matches the result from instantiate2_address function
    assert_eq!(parsed.contract_address, contract_human_addr.to_string());
}
