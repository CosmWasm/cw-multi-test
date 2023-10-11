use crate::error::AnyResult;
use crate::{AppResponse, FailingModule, Module};
use cosmwasm_std::{Binary, Empty, IbcMsg, IbcQuery};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

pub trait Ibc: Module<ExecT = IbcMsg, QueryT = IbcQuery, SudoT = Empty> {}

impl Ibc for FailingModule<IbcMsg, IbcQuery, Empty> {}

pub struct IbcAcceptingModule;

impl Module for IbcAcceptingModule {
    type ExecT = IbcMsg;
    type QueryT = IbcQuery;
    type SudoT = Empty;

    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn cosmwasm_std::Api,
        _storage: &mut dyn cosmwasm_std::Storage,
        _router: &dyn crate::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &cosmwasm_std::BlockInfo,
        _sender: cosmwasm_std::Addr,
        _msg: Self::ExecT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: cosmwasm_std::CustomQuery + DeserializeOwned + 'static,
    {
        Ok(AppResponse::default())
    }

    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn cosmwasm_std::Api,
        _storage: &mut dyn cosmwasm_std::Storage,
        _router: &dyn crate::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &cosmwasm_std::BlockInfo,
        _msg: Self::SudoT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: cosmwasm_std::CustomQuery + DeserializeOwned + 'static,
    {
        Ok(AppResponse::default())
    }

    fn query(
        &self,
        _api: &dyn cosmwasm_std::Api,
        _storage: &dyn cosmwasm_std::Storage,
        _querier: &dyn cosmwasm_std::Querier,
        _block: &cosmwasm_std::BlockInfo,
        _request: Self::QueryT,
    ) -> AnyResult<Binary> {
        Ok(Binary::default())
    }
}

impl Ibc for IbcAcceptingModule {}
