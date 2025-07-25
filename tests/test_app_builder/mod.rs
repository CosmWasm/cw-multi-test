use cosmwasm_std::{
    Addr, Api, Binary, BlockInfo, CustomMsg, CustomQuery, Querier, StdError, StdResult, Storage,
};
use cw_multi_test::{AppResponse, CosmosRouter, Module};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::marker::PhantomData;

mod test_default;
mod test_with_api;
mod test_with_bank;
mod test_with_block;
#[cfg(all(feature = "cosmwasm_1_3", feature = "staking"))]
mod test_with_distribution;
#[cfg(feature = "stargate")]
mod test_with_gov;
#[cfg(feature = "stargate")]
mod test_with_ibc;
#[cfg(feature = "staking")]
mod test_with_staking;
#[cfg(feature = "stargate")]
mod test_with_stargate;
mod test_with_storage;
#[cfg(feature = "cosmwasm_1_2")]
mod test_with_wasm;

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

impl<ExecT, QueryT, SudoT> Module for MyKeeper<ExecT, QueryT, SudoT>
where
    ExecT: Debug,
    QueryT: Debug,
    SudoT: Debug,
{
    type ExecT = ExecT;
    type QueryT = QueryT;
    type SudoT = SudoT;

    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _sender: Addr,
        _msg: Self::ExecT,
    ) -> StdResult<AppResponse>
    where
        ExecC: CustomMsg + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        Err(StdError::msg(self.1))
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _request: Self::QueryT,
    ) -> StdResult<Binary> {
        Err(StdError::msg(self.2))
    }

    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _msg: Self::SudoT,
    ) -> StdResult<AppResponse>
    where
        ExecC: CustomMsg + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        Err(StdError::msg(self.3))
    }
}
