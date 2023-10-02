use anyhow::Result as AnyResult;
use cosmwasm_std::{
    Addr, Api, Binary, BlockInfo, CustomQuery, Querier, StakingMsg, StakingQuery, Storage,
};
use cw_multi_test::{AppBuilder, AppResponse, CosmosRouter, Module, Staking, StakingSudo};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

struct MyStakeKeeper {}

impl Module for MyStakeKeeper {
    type ExecT = StakingMsg;
    type QueryT = StakingQuery;
    type SudoT = StakingSudo;

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
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        todo!()
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
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        todo!()
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _request: Self::QueryT,
    ) -> AnyResult<Binary> {
        todo!()
    }
}

impl Staking for MyStakeKeeper {}

#[test]
fn building_app_with_custom_staking_should_work() {
    let app_builder = AppBuilder::default();
    let _ = app_builder
        .with_staking(MyStakeKeeper {})
        .build(|_, _, _| {});
}
