use anyhow::Result as AnyResult;
use cosmwasm_std::{
    Addr, Api, Binary, BlockInfo, Deps, DepsMut, Empty, Env, MessageInfo, Querier, Record,
    Response, StdError, Storage, WasmMsg, WasmQuery,
};
use cw_multi_test::{
    AppBuilder, AppResponse, Contract, ContractData, ContractWrapper, CosmosRouter, Wasm,
};
use std::marker::PhantomData;

mod contracts {
    use super::*;

    pub mod caller {
        use super::*;

        fn instantiate(
            _deps: DepsMut,
            _env: Env,
            _info: MessageInfo,
            _msg: Empty,
        ) -> Result<Response, StdError> {
            unimplemented!()
        }

        fn execute(
            _deps: DepsMut,
            _env: Env,
            _info: MessageInfo,
            _msg: WasmMsg,
        ) -> Result<Response, StdError> {
            unimplemented!()
        }

        fn query(_deps: Deps, _env: Env, _msg: Empty) -> Result<Binary, StdError> {
            unimplemented!()
        }

        pub fn contract() -> Box<dyn Contract<Empty>> {
            Box::new(ContractWrapper::new_with_empty(execute, instantiate, query))
        }
    }
}

#[test]
fn building_app_with_custom_wasm_should_work() {
    struct MyWasm<ExecT, QueryT>(PhantomData<(ExecT, QueryT)>);

    impl<ExecT, QueryT> Default for MyWasm<ExecT, QueryT> {
        fn default() -> Self {
            Self(PhantomData)
        }
    }

    impl<ExecT, QueryT> Wasm<ExecT, QueryT> for MyWasm<ExecT, QueryT> {
        fn query(
            &self,
            _api: &dyn Api,
            _storage: &dyn Storage,
            _querier: &dyn Querier,
            _block: &BlockInfo,
            _request: WasmQuery,
        ) -> AnyResult<Binary> {
            unimplemented!()
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
            unimplemented!()
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
            unimplemented!()
        }

        fn store_code(&mut self, _creator: Addr, _code: Box<dyn Contract<ExecT, QueryT>>) -> u64 {
            154
        }

        fn duplicate_code(&mut self, _code_id: u64) -> AnyResult<u64> {
            unimplemented!()
        }

        fn contract_data(
            &self,
            _storage: &dyn Storage,
            _address: &Addr,
        ) -> AnyResult<ContractData> {
            unimplemented!()
        }

        fn dump_wasm_raw(&self, _storage: &dyn Storage, _address: &Addr) -> Vec<Record> {
            unimplemented!()
        }
    }

    let app_builder = AppBuilder::default();
    let mut app = app_builder.with_wasm(MyWasm::default()).build(|_, _, _| {});
    let code_id = app.store_code(contracts::caller::contract());
    assert_eq!(154, code_id);
}
