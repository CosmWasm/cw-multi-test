#[test]
#[cfg(feature = "cosmwasm_1_2")]
fn required_entrypoints_should_work() {
    use cosmwasm_std::{Checksum, Empty};
    use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor, IntoAddr};

    // Contract definition. Contains only required entrypoints.
    mod the_contract {
        use cosmwasm_std::{
            to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
        };

        pub fn instantiate(
            _deps: DepsMut,
            _env: Env,
            _info: MessageInfo,
            _msg: Empty,
        ) -> StdResult<Response> {
            Ok(Response::default())
        }

        pub fn execute(
            _deps: DepsMut,
            _env: Env,
            _info: MessageInfo,
            _msg: Empty,
        ) -> StdResult<Response> {
            Ok(Response::default())
        }

        pub fn query(_deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
            to_json_binary(&Empty {})
        }
    }

    // Returns the wrapped contract with simulated checksum.
    pub fn contract() -> Box<dyn Contract<Empty>> {
        Box::new(
            ContractWrapper::new(
                the_contract::execute,
                the_contract::instantiate,
                the_contract::query,
            )
            .with_checksum(Checksum::generate(&[1, 2, 3, 4, 5, 6, 7, 8, 9])),
        )
    }

    // Create the contract wrapper.
    let contract = contract();

    // Save checksum for later use.
    let checksum = contract.checksum();

    // Initialize the chain.
    let mut app = App::default();

    // Store the contract code on chain.
    let code_id = app.store_code(contract);

    assert_eq!(1, code_id);

    // Prepare addresses.
    let owner_addr = "owner".into_addr();
    let admin_addr = "admin".into_addr();

    // Calling `instantiate` entrypoint should work.
    let contract_addr = app
        .instantiate_contract(
            code_id,
            owner_addr.clone(),
            &Empty {},
            &[],
            "the-contract",
            Some(admin_addr.to_string()),
        )
        .unwrap();

    // Calling `execute` entrypoint should work.
    let _: AppResponse = app
        .execute_contract(owner_addr.clone(), contract_addr.clone(), &Empty {}, &[])
        .unwrap();

    // Calling `query` entrypoint should work.
    let _: Empty = app
        .wrap()
        .query_wasm_smart(contract_addr.clone(), &Empty {})
        .unwrap();

    // Querying checksum should work.
    let code_info_response = app.wrap().query_wasm_code_info(code_id).unwrap();
    assert_eq!(
        checksum.unwrap().as_slice(),
        code_info_response.checksum.as_slice()
    );

    // Calling `sudo` entrypoint should fail, because is not implemented in the contract.
    let res = app.wasm_sudo(contract_addr.clone(), &Empty {});
    let err = res.err().unwrap().root_cause().to_string();
    assert_eq!("sudo is not implemented for contract", err);

    // Calling `migrate` entrypoint should now fail because we point the non-existing new code id.
    let res = app.migrate_contract(owner_addr.clone(), contract_addr.clone(), &Empty {}, 2);
    let err = res.err().unwrap().root_cause().to_string();
    assert_eq!("Cannot migrate contract to unregistered code id", err);

    // Calling `migrate` entrypoint should now fail because the owner is not an admin.
    let res = app.migrate_contract(owner_addr, contract_addr.clone(), &Empty {}, 1);
    let err = res.err().unwrap().root_cause().to_string();
    assert!(err.starts_with("Only admin can migrate contract: "));

    // Calling `migrate` entrypoint should now fail, because it is not implemented in contract.
    let res = app.migrate_contract(admin_addr, contract_addr, &Empty {}, 1);
    let err = res.err().unwrap().root_cause().to_string();
    assert_eq!("migrate is not implemented for contract", err);
}
