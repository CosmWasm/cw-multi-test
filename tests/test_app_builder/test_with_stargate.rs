use anyhow::bail;
use cosmwasm_std::{
    to_json_vec, Addr, Api, Binary, BlockInfo, CosmosMsg, CustomQuery, Empty, Querier,
    QueryRequest, Storage,
};
use cw_multi_test::error::AnyResult;
use cw_multi_test::{
    AppBuilder, AppResponse, CosmosRouter, Executor, Stargate, StargateAccepting, StargateFailing,
};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

const EXECUTE_MSG: &str = "stargate execute called";
const QUERY_MSG: &str = "stargate query called";

struct MyStargateKeeper;

impl Stargate for MyStargateKeeper {
    /// Custom processing of stargate messages.
    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _sender: Addr,
        type_url: String,
        value: Binary,
    ) -> AnyResult<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        assert_eq!("test", type_url);
        assert_eq!(Binary::default(), value);
        bail!(EXECUTE_MSG);
    }

    /// Custom stargate queries.
    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        path: String,
        data: Binary,
    ) -> AnyResult<Binary> {
        assert_eq!("test", path);
        assert_eq!(Binary::default(), data);
        bail!(QUERY_MSG);
    }
}

#[test]
fn building_app_with_custom_stargate_should_work() {
    // build custom stargate keeper
    let stargate_keeper = MyStargateKeeper;

    // build the application with custom stargate keeper
    let app_builder = AppBuilder::default();
    let mut app = app_builder
        .with_stargate(stargate_keeper)
        .build(|_, _, _| {});

    // executing stargate message should return
    // an error defined in custom stargate keeper
    assert_eq!(
        app.execute(
            Addr::unchecked("sender"),
            CosmosMsg::Stargate {
                type_url: "test".to_string(),
                value: Default::default()
            },
        )
        .unwrap_err()
        .to_string(),
        EXECUTE_MSG,
    );

    // executing stargate query should return
    // an error defined in custom stargate keeper
    let query: QueryRequest<Empty> = QueryRequest::Stargate {
        path: "test".to_string(),
        data: Default::default(),
    };
    assert_eq!(
        app.wrap()
            .raw_query(to_json_vec(&query).unwrap().as_slice())
            .unwrap()
            .unwrap_err(),
        QUERY_MSG
    );
}

#[test]
fn building_app_with_accepting_stargate_should_work() {
    let app_builder = AppBuilder::default();
    let mut app = app_builder
        .with_stargate(StargateAccepting)
        .build(|_, _, _| {});

    app.execute(
        Addr::unchecked("sender"),
        CosmosMsg::Stargate {
            type_url: "test".to_string(),
            value: Default::default(),
        },
    )
    .unwrap();

    let _ = app
        .wrap()
        .query::<Empty>(&QueryRequest::Stargate {
            path: "test".to_string(),
            data: Default::default(),
        })
        .is_ok();
}

#[test]
fn building_app_with_failing_stargate_should_work() {
    let app_builder = AppBuilder::default();
    let mut app = app_builder
        .with_stargate(StargateFailing)
        .build(|_, _, _| {});

    app.execute(
        Addr::unchecked("sender"),
        CosmosMsg::Stargate {
            type_url: "test".to_string(),
            value: Default::default(),
        },
    )
    .unwrap_err();

    let _ = app
        .wrap()
        .query::<Empty>(&QueryRequest::Stargate {
            path: "test".to_string(),
            data: Default::default(),
        })
        .unwrap_err();
}
