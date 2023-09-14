use cosmwasm_std::{Binary, Empty};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{FailingModule, Module};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StargateMsg {
    pub type_url: String,
    pub value: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StargateQuery {
    pub path: String,
    pub data: Binary,
}

pub trait Stargate: Module<ExecT = StargateMsg, QueryT = StargateQuery, SudoT = Empty> {}

impl Stargate for FailingModule<StargateMsg, StargateQuery, Empty> {}

#[cfg(feature = "stargate")]
#[cfg(test)]
mod testing {
    use std::fmt::Debug;

    use anyhow::Result as AnyResult;
    use cosmwasm_std::{
        to_binary, Addr, Api, BlockInfo, CosmosMsg, CustomQuery, Deps, DepsMut, Env, MessageInfo,
        Querier, QueryRequest, Response, StdResult, Storage,
    };
    use serde::de::DeserializeOwned;

    use crate::app::no_init;
    use crate::{AppResponse, BasicAppBuilder, ContractWrapper, CosmosRouter, Executor};

    use super::*;

    const RESPONSE_DATA: &str = "text";

    #[derive(Serialize, Deserialize, JsonSchema)]
    struct ExampleQueryResponse(String);

    struct AcceptingModule;

    impl Module for AcceptingModule {
        type ExecT = StargateMsg;
        type QueryT = StargateQuery;
        type SudoT = Empty;

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
            Ok(AppResponse::default())
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
            unimplemented!("sudo is not implemented")
        }

        fn query(
            &self,
            _api: &dyn Api,
            _storage: &dyn Storage,
            _querier: &dyn Querier,
            _block: &BlockInfo,
            _request: Self::QueryT,
        ) -> AnyResult<Binary> {
            Ok(to_binary(&ExampleQueryResponse(RESPONSE_DATA.to_string()))?)
        }
    }

    impl Stargate for AcceptingModule {}

    #[test]
    fn init_with_stargate() {
        let mut app = BasicAppBuilder::new()
            .with_stargate(AcceptingModule {})
            .build(no_init);

        let sender = Addr::unchecked("sender");
        let msg = CosmosMsg::Stargate {
            type_url: "/this.is.a.stargate.test".to_string(),
            value: Default::default(),
        };
        app.execute(sender, msg).unwrap();

        let req = QueryRequest::Stargate {
            path: "/cosmos.bank.v1beta1.Query/TotalSupply".to_string(),
            data: Default::default(),
        };
        let resp: ExampleQueryResponse = app.wrap().query(&req).unwrap();
        assert_eq!(resp.0, RESPONSE_DATA)
    }

    #[test]
    fn test_contract_producing_sgate_msgs() {
        fn execute(deps: DepsMut, _: Env, _: MessageInfo, _: Empty) -> StdResult<Response> {
            let res: ExampleQueryResponse = deps.querier.query(&QueryRequest::Stargate {
                path: "/cosmos.bank.v1beta1.Query/TotalSupply".to_string(),
                data: Default::default(),
            })?;
            Ok(Response::new().add_message(CosmosMsg::Stargate {
                type_url: "/this.is.a.stargate.test".to_string(),
                value: to_binary(&res.0)?,
            }))
        }
        fn query(deps: Deps, _: Env, _: Empty) -> StdResult<Binary> {
            let res: ExampleQueryResponse = deps.querier.query(&QueryRequest::Stargate {
                path: "/cosmos.bank.v1beta1.Query/TotalSupply".to_string(),
                data: Default::default(),
            })?;
            to_binary(&res.0)
        }
        let contract = Box::new(ContractWrapper::new(execute, execute, query));

        let mut app = BasicAppBuilder::new()
            .with_stargate(AcceptingModule {})
            .build(no_init);

        let code_id = app.store_code(contract);
        let sender = Addr::unchecked("sender");
        let contract_addr = app
            .instantiate_contract(code_id, sender.clone(), &Empty {}, &[], "contract", None)
            .unwrap();

        // Stargate message produced by contract should be accepted
        app.execute_contract(sender, contract_addr.clone(), &Empty {}, &[])
            .unwrap();

        // Contract just simply forwards the query response data
        let resp: String = app
            .wrap()
            .query_wasm_smart(contract_addr, &Empty {})
            .unwrap();
        assert_eq!(resp, RESPONSE_DATA);
    }
}
