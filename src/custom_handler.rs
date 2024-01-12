//! # Custom message and query handler

use crate::app::CosmosRouter;
use crate::error::{bail, AnyResult};
use crate::{AppResponse, Module};
use cosmwasm_std::{Addr, Api, Binary, BlockInfo, Empty, Querier, Storage};
use derivative::Derivative;
use std::cell::{Ref, RefCell};
use std::ops::Deref;
use std::rc::Rc;

/// A cache for messages and queries processes by the custom module.
#[derive(Derivative)]
#[derivative(Default(bound = "", new = "true"), Clone(bound = ""))]
pub struct CachingCustomHandlerState<ExecC, QueryC> {
    /// Cache for processes custom messages.
    execs: Rc<RefCell<Vec<ExecC>>>,
    /// Cache for processed custom queries.
    queries: Rc<RefCell<Vec<QueryC>>>,
}

impl<ExecC, QueryC> CachingCustomHandlerState<ExecC, QueryC> {
    /// Returns a slice of processed custom messages.
    pub fn execs(&self) -> impl Deref<Target = [ExecC]> + '_ {
        Ref::map(self.execs.borrow(), Vec::as_slice)
    }

    /// Returns a slice of processed custom queries.
    pub fn queries(&self) -> impl Deref<Target = [QueryC]> + '_ {
        Ref::map(self.queries.borrow(), Vec::as_slice)
    }

    /// Clears the cache.
    pub fn reset(&self) {
        self.execs.borrow_mut().clear();
        self.queries.borrow_mut().clear();
    }
}

/// Custom handler that stores all received messages and queries.
///
/// State is thin shared state, so it can be hold after mock is passed to [App](crate::App) to read state.
#[derive(Clone, Derivative)]
#[derivative(Default(bound = "", new = "true"))]
pub struct CachingCustomHandler<ExecC, QueryC> {
    /// Cached state.
    state: CachingCustomHandlerState<ExecC, QueryC>,
}

impl<ExecC, QueryC> CachingCustomHandler<ExecC, QueryC> {
    /// Returns the cached state.
    pub fn state(&self) -> CachingCustomHandlerState<ExecC, QueryC> {
        self.state.clone()
    }
}

impl<Exec, Query> Module for CachingCustomHandler<Exec, Query> {
    type ExecT = Exec;
    type QueryT = Query;
    type SudoT = Empty;

    // TODO: how to assert like `where ExecC: Exec, QueryC: Query`
    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _sender: Addr,
        msg: Self::ExecT,
    ) -> AnyResult<AppResponse> {
        self.state.execs.borrow_mut().push(msg);
        Ok(AppResponse::default())
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        request: Self::QueryT,
    ) -> AnyResult<Binary> {
        self.state.queries.borrow_mut().push(request);
        Ok(Binary::default())
    }

    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        msg: Self::SudoT,
    ) -> AnyResult<AppResponse> {
        bail!("Unexpected custom sudo message {:?}", msg)
    }
}
