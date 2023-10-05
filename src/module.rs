//! # Module interface.

use crate::app::CosmosRouter;
use crate::AnyResult;
use crate::AppResponse;
use anyhow::bail;
use cosmwasm_std::{Addr, Api, Binary, BlockInfo, CustomQuery, Querier, Storage};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::marker::PhantomData;

/// Module interface.
pub trait Module {
    /// Type of the messages processes by [execute](Self::execute) function.
    type ExecT;
    /// Type of the messages processes by [query](Self::query) function.
    type QueryT;
    /// Type of the messages processes by [sudo](Self::sudo) function.
    type SudoT;

    /// Runs any [ExecT](Self::ExecT) message,
    /// which can be called by any external actor or smart contract.
    fn execute<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: Self::ExecT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static;

    /// Runs any [QueryT](Self::QueryT) message,
    /// which can be called by any external actor or smart contract.
    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        request: Self::QueryT,
    ) -> AnyResult<Binary>;

    /// Runs any [SudoT](Self::SudoT) privileged action,
    /// like minting tokens or governance proposals.
    ///
    /// This allows modules to have full access to privileged actions,
    /// that cannot be triggered by smart contracts.
    /// There is no sender, as this must be called with previous authorization.
    fn sudo<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        msg: Self::SudoT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static;
}

/// Module implementation that fails when any of the public function is called.
pub struct FailingModule<ExecT, QueryT, SudoT>(PhantomData<(ExecT, QueryT, SudoT)>);

impl<ExecT, QueryT, SudoT> FailingModule<ExecT, QueryT, SudoT> {
    /// Creates a new failing module.
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<ExecT, QueryT, SudoT> Default for FailingModule<ExecT, QueryT, SudoT> {
    /// Creates a failing module with default settings.
    fn default() -> Self {
        Self::new()
    }
}

impl<ExecT, QueryT, SudoT> Module for FailingModule<ExecT, QueryT, SudoT>
where
    ExecT: Debug,
    QueryT: Debug,
    SudoT: Debug,
{
    type ExecT = ExecT;
    type QueryT = QueryT;
    type SudoT = SudoT;

    /// Runs any [ExecT](Self::ExecT) message, always returns an error.
    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        sender: Addr,
        msg: Self::ExecT,
    ) -> AnyResult<AppResponse> {
        bail!("Unexpected exec msg {:?} from {:?}", msg, sender)
    }

    /// Runs any [QueryT](Self::QueryT) message, always returns an error.
    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        request: Self::QueryT,
    ) -> AnyResult<Binary> {
        bail!("Unexpected custom query {:?}", request)
    }

    /// Runs any [SudoT](Self::SudoT) privileged action, always returns an error.
    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        msg: Self::SudoT,
    ) -> AnyResult<AppResponse> {
        bail!("Unexpected sudo msg {:?}", msg)
    }
}
