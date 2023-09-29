use anyhow::Result as AnyResult;
use cosmwasm_std::{
    Addr, Api, Binary, BlockInfo, Deps, DepsMut, Empty, Env, MessageInfo, Querier, Record,
    Response, StdError, Storage, SubMsg, WasmMsg, WasmQuery,
};
use cw_multi_test::{
    AppBuilder, AppResponse, Contract, ContractData, ContractWrapper, CosmosRouter, Wasm,
};
use schemars::JsonSchema;
use std::fmt::Debug;
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
            Ok(Response::default())
        }

        fn execute(
            _deps: DepsMut,
            _env: Env,
            _info: MessageInfo,
            msg: WasmMsg,
        ) -> Result<Response, StdError> {
            let message = SubMsg::new(msg);

            Ok(Response::new().add_submessage(message))
        }

        fn query(_deps: Deps, _env: Env, _msg: Empty) -> Result<Binary, StdError> {
            Err(StdError::generic_err(
                "query not implemented for the `caller` contract",
            ))
        }

        pub fn contract<C>() -> Box<dyn Contract<C>>
        where
            C: Clone + Debug + PartialEq + JsonSchema + 'static,
        {
            let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
            Box::new(contract)
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
            todo!()
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
            todo!()
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
            todo!()
        }

        fn store_code(&mut self, _creator: Addr, _code: Box<dyn Contract<ExecT, QueryT>>) -> u64 {
            154
        }

        fn duplicate_code(&mut self, _code_id: u64) -> AnyResult<u64> {
            todo!()
        }

        fn contract_data(
            &self,
            _storage: &dyn Storage,
            _address: &Addr,
        ) -> AnyResult<ContractData> {
            todo!()
        }

        fn dump_wasm_raw(&self, _storage: &dyn Storage, _address: &Addr) -> Vec<Record> {
            todo!()
        }
    }

    let app_builder = AppBuilder::default();
    let mut app = app_builder.with_wasm(MyWasm::default()).build(|_, _, _| {});
    let code_id = app.store_code(contracts::caller::contract());
    assert_eq!(154, code_id);
}
