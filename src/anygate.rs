//! # Handler for `CosmosMsg::Stargate`, `CosmosMsg::Any`, `QueryRequest::Stargate` and `QueryRequest::Grpc` messages

use crate::error::AnyResult;
use crate::{AppResponse, CosmosRouter};
use anyhow::bail;
use cosmwasm_std::{
    Addr, AnyMsg, Api, Binary, BlockInfo, CustomMsg, CustomQuery, GrpcQuery, Querier, Storage,
};
use serde::de::DeserializeOwned;

/// Interface of handlers for processing `Stargate`/`Any` message variants
/// and `Stargate`/`Grpc` queries.
pub trait Anygate {
    /// Processes `CosmosMsg::Stargate` message variant.
    fn execute_stargate<ExecC, QueryC>(
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
        ExecC: CustomMsg + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        let _ = (api, storage, router, block);
        bail!(
            "Unexpected execute from {}, type_url={}, value={}",
            sender,
            type_url,
            value
        )
    }

    /// Processes `CosmosMsg::Any` message variant.
    fn execute_any<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: AnyMsg,
    ) -> AnyResult<AppResponse>
    where
        ExecC: CustomMsg + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        let _ = (api, storage, router, block);
        bail!("Unexpected execute from {}, msg={:?}", sender, msg,)
    }

    /// Processes `QueryRequest::Stargate` query.
    fn query_stargate(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        path: String,
        data: Binary,
    ) -> AnyResult<Binary> {
        let _ = (api, storage, querier, block);
        bail!("Unexpected query, path={}, data={}", path, data)
    }

    /// Processes `QueryRequest::Grpc` query.
    fn query_grpc(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        request: GrpcQuery,
    ) -> AnyResult<Binary> {
        let _ = (api, storage, querier, block);
        bail!("Unexpected query, request={:?}", request)
    }
}

/// Always failing handler for `Stargate`/`Any` message variants
/// and `Stargate`/`Grpc` queries.
pub struct FailingAnygate;

impl Anygate for FailingAnygate {}
