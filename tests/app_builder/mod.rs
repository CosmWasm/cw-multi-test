use anyhow::{bail, Result as AnyResult};
use cosmwasm_std::{Addr, Api, Binary, BlockInfo, CustomQuery, Querier, Storage};
use cw_multi_test::{AppResponse, CosmosRouter, Module};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::marker::PhantomData;

mod with_bank;
mod with_block;
mod with_distribution;
mod with_staking;
mod with_storage;
mod with_wasm;

struct MyKeeper<ExecT, QueryT, SudoT>(
    PhantomData<(ExecT, QueryT, SudoT)>,
    &'static str,
    &'static str,
    &'static str,
);

impl<ExecT, QueryT, SudoT> MyKeeper<ExecT, QueryT, SudoT> {
    fn new(execute_msg: &'static str, query_msg: &'static str, sudo_msg: &'static str) -> Self {
        Self(Default::default(), execute_msg, query_msg, sudo_msg)
    }
}

impl<Exec, Query, Sudo> Module for MyKeeper<Exec, Query, Sudo>
where
    Exec: Debug,
    Query: Debug,
    Sudo: Debug,
{
    type ExecT = Exec;
    type QueryT = Query;
    type SudoT = Sudo;

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
        bail!(self.1);
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
        bail!(self.3);
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _request: Self::QueryT,
    ) -> AnyResult<Binary> {
        bail!(self.2);
    }
}
