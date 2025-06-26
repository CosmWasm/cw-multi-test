use crate::test_app_builder::MyKeeper;
use crate::test_contracts;
use cosmwasm_std::{
    Addr, Api, Binary, BlockInfo, Empty, Querier, Record, StdError, StdResult, Storage, WasmMsg,
    WasmQuery,
};
use cw_multi_test::{
    no_init, AppBuilder, AppResponse, Contract, ContractData, CosmosRouter, Executor, Wasm,
    WasmKeeper, WasmSudo,
};

const EXECUTE_MSG: &str = "wasm execute called";
const QUERY_MSG: &str = "wasm query called";
const SUDO_MSG: &str = "wasm sudo called";
const DUPLICATE_CODE_MSG: &str = "wasm duplicate code called";
const CONTRACT_DATA_MSG: &str = "wasm contract data called";

const CODE_ID: u64 = 154;

/// Utility function that returns a raw WASM code (without any meaning, just for testing purposes).
fn wasm_raw() -> Vec<Record> {
    vec![(vec![154u8], vec![155u8])]
}

// This is on purpose derived from module, to check if there are no compilation errors
// when custom wasm keeper implements also Module trait (although it is not needed).
type MyWasmKeeper = MyKeeper<Empty, Empty, Empty>;

impl<ExecT, QueryT> Wasm<ExecT, QueryT> for MyWasmKeeper {
    fn execute(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecT, QueryC = QueryT>,
        _block: &BlockInfo,
        _sender: Addr,
        _msg: WasmMsg,
    ) -> StdResult<AppResponse> {
        Err(StdError::msg(self.1))
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _request: WasmQuery,
    ) -> StdResult<Binary> {
        Err(StdError::msg(self.2))
    }

    fn sudo(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecT, QueryC = QueryT>,
        _block: &BlockInfo,
        _msg: WasmSudo,
    ) -> StdResult<AppResponse> {
        Err(StdError::msg(self.3))
    }

    fn store_code(&mut self, _creator: Addr, _code: Box<dyn Contract<ExecT, QueryT>>) -> u64 {
        CODE_ID
    }

    fn store_code_with_id(
        &mut self,
        _creator: Addr,
        code_id: u64,
        _code: Box<dyn Contract<ExecT, QueryT>>,
    ) -> StdResult<u64> {
        Ok(code_id)
    }

    fn duplicate_code(&mut self, _code_id: u64) -> StdResult<u64> {
        Err(StdError::msg(DUPLICATE_CODE_MSG))
    }

    fn contract_data(&self, _storage: &dyn Storage, _address: &Addr) -> StdResult<ContractData> {
        Err(StdError::msg(CONTRACT_DATA_MSG))
    }

    fn dump_wasm_raw(&self, _storage: &dyn Storage, _address: &Addr) -> Vec<Record> {
        wasm_raw()
    }
}

#[test]
fn building_app_with_custom_wasm_should_work() {
    // build custom wasm keeper
    let wasm_keeper = MyWasmKeeper::new(EXECUTE_MSG, QUERY_MSG, SUDO_MSG);

    // build the application with custom wasm keeper
    let mut app = AppBuilder::default().with_wasm(wasm_keeper).build(no_init);

    // prepare addresses
    let contract_addr = app.api().addr_make("contract");
    let sender_addr = app.api().addr_make("sender");

    // calling store_code should return value defined in custom keeper
    assert_eq!(CODE_ID, app.store_code(test_contracts::counter::contract()));

    // calling duplicate_code should return error defined in custom keeper
    assert_eq!(
        "kind: Other, error: wasm duplicate code called",
        app.duplicate_code(CODE_ID).unwrap_err().to_string()
    );

    // calling contract_data should return error defined in custom keeper
    assert_eq!(
        "kind: Other, error: wasm contract data called",
        app.contract_data(&contract_addr).unwrap_err().to_string()
    );

    // calling dump_wasm_raw should return value defined in custom keeper
    assert_eq!(wasm_raw(), app.dump_wasm_raw(&contract_addr));

    // executing wasm execute should return an error defined in custom keeper
    assert_eq!(
        "kind: Other, error: wasm execute called",
        app.execute(
            sender_addr,
            WasmMsg::Instantiate {
                admin: None,
                code_id: 0,
                msg: Default::default(),
                funds: vec![],
                label: "".to_string(),
            }
            .into()
        )
        .unwrap_err()
        .to_string()
    );

    // executing wasm sudo should return an error defined in custom keeper
    assert_eq!(
        "kind: Other, error: wasm sudo called",
        app.sudo(
            WasmSudo {
                contract_addr,
                message: Default::default()
            }
            .into()
        )
        .unwrap_err()
        .to_string()
    );

    // executing wasm query should return an error defined in custom keeper
    assert_eq!(
        format!("kind: Other, error: Querier contract error: kind: Other, error: {QUERY_MSG}"),
        app.wrap()
            .query_wasm_code_info(CODE_ID)
            .unwrap_err()
            .to_string()
    );
}

#[test]
fn compiling_with_wasm_keeper_should_work() {
    // this verifies only compilation errors
    // while our WasmKeeper does not implement Module
    let _ = AppBuilder::default()
        .with_wasm(WasmKeeper::default())
        .build(no_init);
}
