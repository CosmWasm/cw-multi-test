use crate::error::AnyResult;
use crate::{AppResponse, CosmosRouter};
use anyhow::bail;
use cosmwasm_std::{Addr, Api, Binary, BlockInfo, CustomQuery, Querier, Storage};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

/// Stargate interface.
///
/// This trait provides the default behavior for all functions
/// that is equal to [StargateFailing] implementation.
pub trait Stargate {
    /// Processes stargate messages.
    ///
    /// The `CosmosMsg::Stargate` message is unwrapped before processing.
    /// The `type_url` and `value` attributes of `CosmosMsg::Stargate`
    /// are passed directly to this handler.
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
    ///
    /// The `QueryRequest::Stargate` query request is unwrapped before processing.
    /// The `path` and `data` attributes of `QueryRequest::Stargate` are passed
    /// directly to this handler.
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
    /// Accepts all stargate messages. Returns default `AppResponse`.
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
        let _ = (api, storage, router, block, sender, type_url, value);
        Ok(AppResponse::default())
    }

    /// Accepts all stargate queries. Returns default `Binary`.
    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        path: String,
        data: Binary,
    ) -> AnyResult<Binary> {
        let _ = (api, storage, querier, block, path, data);
        Ok(Binary::default())
    }
}
