use crate::app_builder::{contracts, MyKeeper};
use cosmwasm_std::{
    Addr, Api, Binary, BlockInfo, Empty, Querier, Record, Storage, WasmMsg, WasmQuery,
};
use cw_multi_test::errors::{bail, AnyResult};
use cw_multi_test::{
    AppBuilder, AppResponse, Contract, ContractData, CosmosRouter, Executor, Wasm, WasmKeeper,
    WasmSudo,
};
use once_cell::sync::Lazy;

const EXECUTE_MSG: &str = "wasm execute called";
const QUERY_MSG: &str = "wasm query called";
const SUDO_MSG: &str = "wasm sudo called";
const DUPLICATE_CODE_MSG: &str = "wasm duplicate code called";
const CONTRACT_DATA_MSG: &str = "wasm contract data called";

const CODE_ID: u64 = 154;

static WASM_RAW: Lazy<Vec<Record>> = Lazy::new(|| vec![(vec![154u8], vec![155u8])]);

// This is on purpose derived from module, to check if there are no compilation errors
// when custom wasm keeper implements also Module trait (although it is not needed).
type MyWasmKeeper = MyKeeper<Empty, Empty, Empty>;

impl<ExecT, QueryT> Wasm<ExecT, QueryT> for MyWasmKeeper {
    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _request: WasmQuery,
    ) -> AnyResult<Binary> {
        bail!(self.2);
    }

    fn execute(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecT, QueryC = QueryT>,
        _block: &BlockInfo,
        _sender: Addr,
        _msg: WasmMsg,
    ) -> AnyResult<AppResponse> {
        bail!(self.1);
    }

    fn sudo(
        &self,
        _api: &dyn Api,
        _contract_addr: Addr,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecT, QueryC = QueryT>,
        _block: &BlockInfo,
        _msg: Binary,
    ) -> AnyResult<AppResponse> {
        bail!(self.3);
    }

    fn store_code(&mut self, _creator: Addr, _code: Box<dyn Contract<ExecT, QueryT>>) -> u64 {
        CODE_ID
    }

    fn duplicate_code(&mut self, _code_id: u64) -> AnyResult<u64> {
        bail!(DUPLICATE_CODE_MSG);
    }

    fn contract_data(&self, _storage: &dyn Storage, _address: &Addr) -> AnyResult<ContractData> {
        bail!(CONTRACT_DATA_MSG);
    }

    fn dump_wasm_raw(&self, _storage: &dyn Storage, _address: &Addr) -> Vec<Record> {
        WASM_RAW.clone()
    }
}

#[test]
fn building_app_with_custom_wasm_should_work() {
    // build custom wasm keeper
    let wasm_keeper = MyWasmKeeper::new(EXECUTE_MSG, QUERY_MSG, SUDO_MSG);

    // build the application with custom wasm keeper
    let app_builder = AppBuilder::default();
    let mut app = app_builder.with_wasm(wasm_keeper).build(|_, _, _| {});

    // prepare additional input data
    let contract_addr = Addr::unchecked("contract");

    // calling store_code should return value defined in custom keeper
    assert_eq!(CODE_ID, app.store_code(contracts::counter::contract()));

    // calling duplicate_code should return error defined in custom keeper
    assert_eq!(
        DUPLICATE_CODE_MSG,
        app.duplicate_code(CODE_ID).unwrap_err().to_string()
    );

    // calling contract_data should return error defined in custom keeper
    assert_eq!(
        CONTRACT_DATA_MSG,
        app.contract_data(&contract_addr).unwrap_err().to_string()
    );

    // calling dump_wasm_raw should return value defined in custom keeper
    assert_eq!(*WASM_RAW, app.dump_wasm_raw(&contract_addr));

    // executing wasm execute should return an error defined in custom keeper
    assert_eq!(
        EXECUTE_MSG,
        app.execute(
            Addr::unchecked("sender"),
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
        SUDO_MSG,
        app.sudo(
            WasmSudo {
                contract_addr,
                msg: Default::default()
            }
            .into()
        )
        .unwrap_err()
        .to_string()
    );

    // executing wasm query should return an error defined in custom keeper
    #[cfg(feature = "cosmwasm_1_2")]
    assert_eq!(
        format!("Generic error: Querier contract error: {}", QUERY_MSG),
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
    let app_builder = AppBuilder::default();
    let _ = app_builder
        .with_wasm(WasmKeeper::default())
        .build(|_, _, _| {});
}
