use crate::test_contracts::counter;
use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{instantiate2_address, to_json_binary, Api, Empty, WasmMsg};
use cw_multi_test::{AppBuilder, Executor};
use cw_utils::parse_instantiate_response_data;

#[test]
fn instantiate2_works() {
    // prepare the application with custom Api and custom address generator
    let mut app = AppBuilder::default()
        .with_api(MockApi::default().with_prefix("juno"))
        .build_no_init();

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

#[test]
fn instantiate2_should_work_for_multiple_salts() {
    // prepare the application with custom Api and custom address generator
    let mut app = AppBuilder::default()
        .with_api(MockApi::default().with_prefix("juno"))
        .build_no_init();

    // prepare addresses for sender and creator
    let sender = app.api().addr_make("sender");
    let creator = app.api().addr_make("creator");

    // store the contract's code
    let code_id = app.store_code_with_creator(creator, counter::contract());

    let mut f = |salt: &str| {
        // instantiate the contract with predictable address and provided salt, sender is the same
        let msg = WasmMsg::Instantiate2 {
            admin: None,
            code_id,
            msg: to_json_binary(&Empty {}).unwrap(),
            funds: vec![],
            label: "label".into(),
            salt: salt.as_bytes().into(),
        };
        let res = app.execute(sender.clone(), msg.into()).unwrap();
        let parsed = parse_instantiate_response_data(res.data.unwrap().as_slice()).unwrap();
        parsed.contract_address
    };

    // make sure, addresses generated for different salts are different
    assert_ne!(f("bad kids 1"), f("bad kids 2"))
}

#[test]
fn instantiate2_fails_for_duplicated_addresses() {
    // prepare the application with custom Api and custom address generator
    let mut app = AppBuilder::default()
        .with_api(MockApi::default().with_prefix("osmo"))
        .build_no_init();

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
    let res = app.execute(sender.clone(), msg.clone().into()).unwrap();

    // check the instantiate result
    let parsed = parse_instantiate_response_data(res.data.unwrap().as_slice()).unwrap();
    assert!(parsed.data.is_none());

    // check the resulting predictable contract's address
    assert_eq!(
        parsed.contract_address,
        "osmo1navvz5rjlvn43xjqxlpl7dunk6hglmhuh7c6a53eq6qamfam3dusg94p04"
    );

    // creating a new instance of the same contract with the same sender and salt
    // should fail because the generated contract address is the same
    app.execute(sender.clone(), msg.into()).unwrap_err();

    // ----------------------------------------------------------------------
    // Below is an additional check, proving that the predictable address
    // from contract instantiation is exactly the same when used with the
    // cosmwasm_std::instantiate2_address twice (same sender and salt).
    // ----------------------------------------------------------------------

    // get the code info of the contract
    let code_info_response = app.wrap().query_wasm_code_info(code_id).unwrap();

    // retrieve the contract's code checksum
    let checksum = code_info_response.checksum.as_slice();

    // canonicalize the sender address (which is now in human Bech32 format)
    let sender_addr = app.api().addr_canonicalize(sender.as_str()).unwrap();

    // get the contract address using cosmwasm_std::instantiate2_address function twice
    let contract_addr_1 = instantiate2_address(checksum, &sender_addr, salt).unwrap();
    let contract_addr_2 = instantiate2_address(checksum, &sender_addr, salt).unwrap();

    // contract addresses should be the same
    assert_eq!(contract_addr_1, contract_addr_2);
}
