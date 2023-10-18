#![cfg(feature = "cosmwasm_1_2")]

use crate::test_contracts;
use cosmwasm_std::{Addr, Empty, HexBinary};
use cw_multi_test::{App, AppBuilder, ChecksumGenerator, WasmKeeper};

#[test]
fn default_checksum_generator_should_work() {
    // prepare default application with default wasm keeper
    let mut app = App::default();

    // store contract's code
    let code_id = app.store_code_with_creator(
        Addr::unchecked("creator"),
        test_contracts::counter::contract(),
    );

    // get code info
    let code_info_response = app.wrap().query_wasm_code_info(code_id).unwrap();

    // this should be default checksum
    assert_eq!(
        code_info_response.checksum.to_hex(),
        "27095b438f70aed35405149bc5e8dfa1d461f7cd9c25359807ad66dcc1396fc7"
    );
}

struct MyChecksumGenerator;

impl ChecksumGenerator for MyChecksumGenerator {
    fn checksum(&self, _creator: &Addr, _code_id: u64) -> HexBinary {
        HexBinary::from_hex("c0ffee01c0ffee02c0ffee03c0ffee04c0ffee05c0ffee06c0ffee07c0ffee08")
            .unwrap()
    }
}

#[test]
fn custom_checksum_generator_should_work() {
    // prepare wasm keeper with custom checksum generator
    let wasm_keeper: WasmKeeper<Empty, Empty> =
        WasmKeeper::default().with_checksum_generator(MyChecksumGenerator);

    // prepare application with custom wasm keeper
    let mut app = AppBuilder::default()
        .with_wasm(wasm_keeper)
        .build(|_, _, _| {});

    // store contract's code
    let code_id = app.store_code_with_creator(
        Addr::unchecked("creator"),
        test_contracts::counter::contract(),
    );

    // get code info
    let code_info_response = app.wrap().query_wasm_code_info(code_id).unwrap();

    // this should be custom checksum
    assert_eq!(
        code_info_response.checksum.to_hex(),
        "c0ffee01c0ffee02c0ffee03c0ffee04c0ffee05c0ffee06c0ffee07c0ffee08"
    );
}
