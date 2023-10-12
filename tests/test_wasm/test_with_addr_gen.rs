use crate::test_api::MockApiBech32;
use crate::test_contracts;
use cosmwasm_std::{instantiate2_address, Addr, Api, CanonicalAddr, Empty, Storage};
use cw_multi_test::error::AnyResult;
use cw_multi_test::{AddressGenerator, AppBuilder, Executor, WasmKeeper};
use sha2::{Digest, Sha256};

#[derive(Default)]
struct TestAddressGenerator;

impl AddressGenerator for TestAddressGenerator {
    fn classic_contract_address(
        &self,
        api: &dyn Api,
        _storage: &mut dyn Storage,
        _code_id: u64,
        instance_id: u64,
    ) -> Addr {
        let digest = Sha256::digest(format!("contract{}", instance_id)).to_vec();
        let canonical_addr = CanonicalAddr::from(digest);
        Addr::unchecked(api.addr_humanize(&canonical_addr).unwrap())
    }

    fn predictable_contract_address(
        &self,
        api: &dyn Api,
        _storage: &mut dyn Storage,
        _code_id: u64,
        _instance_id: u64,
        checksum: &[u8],
        creator: &CanonicalAddr,
        salt: &[u8],
    ) -> AnyResult<Addr> {
        Ok(Addr::unchecked(api.addr_humanize(
            &instantiate2_address(checksum, creator, salt)?,
        )?))
    }
}

#[test]
fn classic_contract_address_should_work() {
    // prepare wasm module with custom address generator
    let wasm_keeper: WasmKeeper<Empty, Empty> =
        WasmKeeper::new_with_custom_address_generator(TestAddressGenerator);

    // prepare application with custom api
    let mut app = AppBuilder::default()
        .with_api(MockApiBech32::new("juno"))
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
}

#[test]
#[cfg(feature = "cosmwasm_1_2")]
fn predictable_contract_address_should_work() {
    // prepare wasm module with custom address generator
    let wasm_keeper: WasmKeeper<Empty, Empty> =
        WasmKeeper::new_with_custom_address_generator(TestAddressGenerator);

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
