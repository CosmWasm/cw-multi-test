use crate::test_addresses::MockAddressGenerator;
use crate::test_api::MockApiBech32;
use crate::test_contracts;
use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{AppBuilder, Executor, WasmKeeper};

#[test]
fn classic_contract_address_should_work() {
    // prepare wasm module with custom address generator
    let wasm_keeper: WasmKeeper<Empty, Empty> =
        WasmKeeper::new_with_custom_address_generator(MockAddressGenerator);

    // prepare application with custom api
    let mut app = AppBuilder::default()
        .with_api(MockApiBech32::new("purple"))
        .with_wasm(wasm_keeper)
        .build(|_, _, _| {});

    // store contract's code
    let code_id = app.store_code_with_creator(
        Addr::unchecked("creator"),
        test_contracts::counter::contract(),
    );

    let owner = app.api().addr_make("owner");

    let contract_addr_1 = app
        .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Counter", None)
        .unwrap();

    let contract_addr_2 = app
        .instantiate_contract(code_id, owner, &Empty {}, &[], "Counter", None)
        .unwrap();

    // addresses of the two contract instances should be different
    assert_ne!(contract_addr_1, contract_addr_2);

    // make sure that generated addresses are in valid Bech32 encoding
    assert_eq!(
        contract_addr_1.to_string(),
        "purple1mzdhwvvh22wrt07w59wxyd58822qavwkx5lcej7aqfkpqqlhaqfs5efvjk"
    );
    assert_eq!(
        contract_addr_2.to_string(),
        "purple14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9smc2vxm"
    );
}

#[test]
#[cfg(feature = "cosmwasm_1_2")]
fn predictable_contract_address_should_work() {
    // prepare wasm module with custom address generator
    let wasm_keeper: WasmKeeper<Empty, Empty> =
        WasmKeeper::new_with_custom_address_generator(MockAddressGenerator);

    // prepare application with custom api
    let mut app = AppBuilder::default()
        .with_api(MockApiBech32::new("juno"))
        .with_wasm(wasm_keeper)
        .build(|_, _, _| {});

    let creator = app.api().addr_make("creator");

    // store contract's code
    let code_id = app.store_code_with_creator(creator.clone(), test_contracts::counter::contract());

    let contract_addr_1 = app
        .instantiate2_contract(
            code_id,
            creator.clone(),
            &Empty {},
            &[],
            "Counter",
            None,
            [1, 2, 3, 4, 5, 6],
        )
        .unwrap();

    let contract_addr_2 = app
        .instantiate2_contract(
            code_id,
            creator.clone(),
            &Empty {},
            &[],
            "Counter",
            None,
            [11, 12, 13, 14, 15, 16],
        )
        .unwrap();

    // addresses of the two contract instances should be different
    assert_ne!(contract_addr_1, contract_addr_2);

    // instantiating a contract with the same salt should fail
    app.instantiate2_contract(
        code_id,
        creator,
        &Empty {},
        &[],
        "Counter",
        None,
        [1, 2, 3, 4, 5, 6],
    )
    .unwrap_err();
}
