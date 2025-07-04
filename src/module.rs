use crate::app::CosmosRouter;
use crate::error::std_error_bail;
use crate::AppResponse;
use cosmwasm_std::{
    Addr, Api, Binary, BlockInfo, CustomMsg, CustomQuery, Querier, StdError, StdResult, Storage,
};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::marker::PhantomData;

/// # General module
///
/// Provides a generic interface for modules within the test environment.
/// It is essential for creating modular and extensible testing setups,
/// allowing developers to integrate custom functionalities
/// or test specific scenarios.
pub trait Module {
    /// Type of messages processed by the module instance.
    type ExecT;
    /// Type of queries processed by the module instance.
    type QueryT;
    /// Type of privileged messages used by the module instance.
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
    ) -> StdResult<AppResponse>
    where
        ExecC: CustomMsg + DeserializeOwned + 'static,
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
    ) -> StdResult<Binary>;

    /// Runs privileged actions, like minting tokens, or governance proposals.
    /// This allows modules to have full access to these privileged actions,
    /// that cannot be triggered by smart contracts.
    ///
    /// There is no sender, as this must be previously authorized before calling.
    fn sudo<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        msg: Self::SudoT,
    ) -> StdResult<AppResponse>
    where
        ExecC: CustomMsg + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static;
}
/// # Always failing module
///
/// This could be a diagnostic or testing tool within the Cosmos ecosystem,
/// designed to intentionally fail during processing any message, query or privileged action.
pub struct FailingModule<ExecT, QueryT, SudoT>(PhantomData<(ExecT, QueryT, SudoT)>);

impl<ExecT, QueryT, SudoT> FailingModule<ExecT, QueryT, SudoT> {
    /// Creates an instance of a failing module.
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<ExecT, QueryT, SudoT> Default for FailingModule<ExecT, QueryT, SudoT> {
    /// Creates a default instance of a failing module.
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
    ) -> StdResult<AppResponse> {
        std_error_bail!("Unexpected exec msg {:?} from {:?}", msg, sender)
    }

    /// Runs any [QueryT](Self::QueryT) message, always returns an error.
    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        request: Self::QueryT,
    ) -> StdResult<Binary> {
        std_error_bail!("Unexpected custom query {:?}", request)
    }

    /// Runs any [SudoT](Self::SudoT) privileged action, always returns an error.
    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        msg: Self::SudoT,
    ) -> StdResult<AppResponse> {
        std_error_bail!("Unexpected sudo msg {:?}", msg)
    }
}
/// # Always accepting module
///
/// This struct represents a module in the Cosmos ecosystem designed to
/// always accept all processed messages, queries and privileged actions.
pub struct AcceptingModule<ExecT, QueryT, SudoT>(PhantomData<(ExecT, QueryT, SudoT)>);

impl<ExecT, QueryT, SudoT> AcceptingModule<ExecT, QueryT, SudoT> {
    /// Creates an instance of an accepting module.
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<ExecT, QueryT, SudoT> Default for AcceptingModule<ExecT, QueryT, SudoT> {
    /// Creates an instance of an accepting module with default settings.
    fn default() -> Self {
        Self::new()
    }
}

impl<ExecT, QueryT, SudoT> Module for AcceptingModule<ExecT, QueryT, SudoT>
where
    ExecT: Debug,
    QueryT: Debug,
    SudoT: Debug,
{
    type ExecT = ExecT;
    type QueryT = QueryT;
    type SudoT = SudoT;

    /// Runs any [ExecT](Self::ExecT) message, always returns a default response.
    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _sender: Addr,
        _msg: Self::ExecT,
    ) -> StdResult<AppResponse> {
        Ok(AppResponse::default())
    }

    /// Runs any [QueryT](Self::QueryT) message, always returns a default (empty) binary.
    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _request: Self::QueryT,
    ) -> StdResult<Binary> {
        Ok(Binary::default())
    }

    /// Runs any [SudoT](Self::SudoT) privileged action, always returns a default response.
    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _msg: Self::SudoT,
    ) -> StdResult<AppResponse> {
        Ok(AppResponse::default())
    }
}
