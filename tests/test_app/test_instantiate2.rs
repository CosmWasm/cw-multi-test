#![cfg(feature = "cosmwasm_1_2")]

use crate::test_contracts::counter;
use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{instantiate2_address, to_json_binary, Addr, Api, Coin, Empty, WasmMsg};
use cw_multi_test::{no_init, App, AppBuilder, Executor};
use cw_utils::parse_instantiate_response_data;

const FUNDS: Vec<Coin> = vec![];
const SALT: &[u8] = "bad kids".as_bytes();
const LABEL: &str = "label";
const JUNO_1: &str = "juno1navvz5rjlvn43xjqxlpl7dunk6hglmhuh7c6a53eq6qamfam3dus7a220h";
const JUNO_2: &str = "juno1qaygqu9plc7nqqgwt7d6dxhmej2tl0lu20j84l5pnz5p4th4zz5qwd77z5";
const OSMO: &str = "osmo1navvz5rjlvn43xjqxlpl7dunk6hglmhuh7c6a53eq6qamfam3dusg94p04";

#[test]
fn instantiate2_works() {
    // prepare the chain with custom Api and custom address generator
    let mut app = AppBuilder::default()
        .with_api(MockApi::default().with_prefix("juno"))
        .build(no_init);

    // prepare addresses for sender and creator
    let sender = app.api().addr_make("sender");
    let creator = app.api().addr_make("creator");

    // store the contract's code (Wasm blob checksum is generated)
    let code_id = app.store_code_with_creator(creator, counter::contract());

    // instantiate the contract with predictable address
    let init_msg = to_json_binary(&Empty {}).unwrap();
    let msg = WasmMsg::Instantiate2 {
        admin: None,
        code_id,
        msg: init_msg,
        funds: FUNDS,
        label: LABEL.into(),
        salt: SALT.into(),
    };
    let res = app.execute(sender.clone(), msg.into()).unwrap();

    // check the instantiate result
    let parsed = parse_instantiate_response_data(res.data.unwrap().as_slice()).unwrap();
    assert!(parsed.data.is_none());

    // check the resulting predictable contract's address
    assert_eq!(parsed.contract_address, JUNO_1); // must be equal
    assert_ne!(parsed.contract_address, JUNO_2); // must differ

    // comparing with the result from `cosmwasm_std::instantiate2_address` is done here
    compare_with_cosmwasm_vm_address(&app, code_id, &sender, &parsed.contract_address);
}

#[test]
fn instantiate2_works_with_checksum_provided_in_contract() {
    // prepare the chain with custom API and custom address generator
    let mut app = AppBuilder::default()
        .with_api(MockApi::default().with_prefix("juno"))
        .build(no_init);

    // prepare addresses for the sender and creator
    let sender = app.api().addr_make("sender");
    let creator = app.api().addr_make("creator");

    // store the contract's code (Wasm blob checksum is provided in contract)
    let code_id = app.store_code_with_creator(creator, counter::contract_with_checksum());

    // instantiate the contract with predictable address
    let init_msg = to_json_binary(&Empty {}).unwrap();
    let msg = WasmMsg::Instantiate2 {
        admin: None,
        code_id,
        msg: init_msg,
        funds: FUNDS,
        label: LABEL.into(),
        salt: SALT.into(),
    };
    let res = app.execute(sender.clone(), msg.into()).unwrap();

    // check the instantiate result
    let parsed = parse_instantiate_response_data(res.data.unwrap().as_slice()).unwrap();
    assert!(parsed.data.is_none());

    // check the resulting predictable contract's address
    assert_eq!(parsed.contract_address, JUNO_2); // must be equal
    assert_ne!(parsed.contract_address, JUNO_1); // must differ

    // comparing with the result from `cosmwasm_std::instantiate2_address` is done here
    compare_with_cosmwasm_vm_address(&app, code_id, &sender, &parsed.contract_address);
}

#[test]
fn instantiate2_should_work_for_multiple_salts() {
    // prepare the application with custom Api and custom address generator
    let mut app = AppBuilder::default()
        .with_api(MockApi::default().with_prefix("juno"))
        .build(no_init);

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
            funds: FUNDS,
            label: LABEL.into(),
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
        .build(no_init);

    // prepare addresses for sender and creator
    let sender = app.api().addr_make("sender");
    let creator = app.api().addr_make("creator");

    // store the contract's code
    let code_id = app.store_code_with_creator(creator, counter::contract());

    // instantiate the contract with predictable address
    let init_msg = to_json_binary(&Empty {}).unwrap();
    let msg = WasmMsg::Instantiate2 {
        admin: None,
        code_id,
        msg: init_msg,
        funds: FUNDS,
        label: LABEL.into(),
        salt: SALT.into(),
    };
    let res = app.execute(sender.clone(), msg.clone().into()).unwrap();

    // check the instantiate result
    let parsed = parse_instantiate_response_data(res.data.unwrap().as_slice()).unwrap();
    assert!(parsed.data.is_none());

    // check the resulting predictable contract's address
    assert_eq!(parsed.contract_address, OSMO);

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
    let contract_addr_1 = instantiate2_address(checksum, &sender_addr, SALT).unwrap();
    let contract_addr_2 = instantiate2_address(checksum, &sender_addr, SALT).unwrap();

    // contract addresses should be the same
    assert_eq!(contract_addr_1, contract_addr_2);
}

/// Utility function proving that the predictable address from contract instantiation
/// is exactly the same as the address returned from the function `cosmwasm_std::instantiate2_address`.
fn compare_with_cosmwasm_vm_address(app: &App, code_id: u64, sender: &Addr, expected_addr: &str) {
    // get the code info of the contract
    let code_info_response = app.wrap().query_wasm_code_info(code_id).unwrap();

    // retrieve the contract's code checksum
    let checksum = code_info_response.checksum.as_slice();

    // canonicalize the sender address (which is now in human Bech32 format)
    let sender_addr = app.api().addr_canonicalize(sender.as_str()).unwrap();

    // get the contract address using cosmwasm_std::instantiate2_address function
    let contract_addr = instantiate2_address(checksum, &sender_addr, SALT).unwrap();

    // humanize the address of the contract
    let contract_human_addr = app.api().addr_humanize(&contract_addr).unwrap();

    // check if the predictable contract's address matches the result from instantiate2_address function
    assert_eq!(expected_addr, contract_human_addr.to_string());
}
