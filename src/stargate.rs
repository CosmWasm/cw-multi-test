//! add-docs

use crate::error::AnyResult;
use crate::{AppResponse, CosmosRouter};
use anyhow::bail;
use cosmwasm_std::{Addr, Api, Binary, BlockInfo, CustomQuery, Querier, Storage};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

/// Stargate interface.
pub trait Stargate {
    /// Processes stargate messages.
    fn execute<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        type_url: String,
        value: Binary,
    ) -> AnyResult<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        let _ = (api, storage, router, block);
        bail!(
            "Unexpected stargate message: (type_ur = {}, value = {:?}) from {:?}",
            type_url,
            value,
            sender
        )
    }

    /// Processes stargate queries.
    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        path: String,
        data: Binary,
    ) -> AnyResult<Binary> {
        let _ = (api, storage, querier, block);
        bail!(
            "Unexpected stargate query: path = {:?}, data = {:?}",
            path,
            data
        )
    }
}

/// Always failing stargate mock implementation.
pub struct StargateFailing;

impl Stargate for StargateFailing {}

/// Always accepting stargate mock implementation.
pub struct StargateAccepting;

impl Stargate for StargateAccepting {
    /// Accepts all stargate messages.
    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _sender: Addr,
        _type_url: String,
        _value: Binary,
    ) -> AnyResult<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        Ok(AppResponse::default())
    }

    /// Accepts all stargate queries.
    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _path: String,
        _data: Binary,
    ) -> AnyResult<Binary> {
        Ok(Binary::default())
    }
}
