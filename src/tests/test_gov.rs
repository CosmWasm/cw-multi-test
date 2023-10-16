use crate::error::AnyResult;
use crate::test_helpers::{stargate, stargate::ExecMsg};
use crate::{App, AppBuilder, AppResponse, CosmosRouter, Executor, Gov, Module};
use cosmwasm_std::{Addr, Api, Binary, BlockInfo, Empty, GovMsg, Querier, Storage};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

struct AcceptingModule;

impl Module for AcceptingModule {
    type ExecT = GovMsg;
    type QueryT = Empty;
    type SudoT = Empty;

    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _sender: Addr,
        _msg: Self::ExecT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: cosmwasm_std::CustomQuery + DeserializeOwned + 'static,
    {
        Ok(AppResponse::default())
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _request: Self::QueryT,
    ) -> AnyResult<Binary> {
        Ok(Binary::default())
    }

    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _msg: Self::SudoT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + schemars::JsonSchema + DeserializeOwned + 'static,
        QueryC: cosmwasm_std::CustomQuery + DeserializeOwned + 'static,
    {
        Ok(AppResponse::default())
    }
}

impl Gov for AcceptingModule {}

#[test]
fn default_gov() {
    let mut app = App::default();
    let code = app.store_code(stargate::contract());
    let contract = app
        .instantiate_contract(
            code,
            Addr::unchecked("owner"),
            &Empty {},
            &[],
            "contract",
            None,
        )
        .unwrap();

    app.execute_contract(Addr::unchecked("owner"), contract, &ExecMsg::Gov {}, &[])
        .unwrap_err();
}

#[test]
fn substituting_gov() {
    let mut app = AppBuilder::new()
        .with_gov(AcceptingModule)
        .build(|_, _, _| ());
    let code = app.store_code(stargate::contract());
    let contract = app
        .instantiate_contract(
            code,
            Addr::unchecked("owner"),
            &Empty {},
            &[],
            "contract",
            None,
        )
        .unwrap();

    app.execute_contract(Addr::unchecked("owner"), contract, &ExecMsg::Gov {}, &[])
        .unwrap();
}
