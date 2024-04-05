use crate::test_app_builder::{MyKeeper, NO_MESSAGE};
use cosmwasm_std::{to_json_vec, CosmosMsg, Empty, QueryRequest};
use cw_multi_test::{
    no_init, AppBuilder, Executor, Stargate, StargateAcceptingModule, StargateFailingModule,
    StargateMsg, StargateQuery,
};

type MyStargateKeeper = MyKeeper<StargateMsg, StargateQuery, Empty>;

impl Stargate for MyStargateKeeper {}

const EXECUTE_MSG: &str = "stargate execute called";
const QUERY_MSG: &str = "stargate query called";

#[test]
fn building_app_with_custom_stargate_should_work() {
    // build custom stargate keeper
    let stargate_keeper = MyStargateKeeper::new(EXECUTE_MSG, QUERY_MSG, NO_MESSAGE);

    // build the application with custom stargate keeper
    let app_builder = AppBuilder::default();
    let mut app = app_builder.with_stargate(stargate_keeper).build(no_init);

    // prepare user addresses
    let sender_addr = app.api().addr_make("sender");

    // executing stargate message should return
    // an error defined in custom stargate keeper
    #[allow(deprecated)]
    let msg = CosmosMsg::Stargate {
        type_url: "test".to_string(),
        value: Default::default(),
    };
    assert_eq!(
        EXECUTE_MSG,
        app.execute(sender_addr, msg,).unwrap_err().to_string(),
    );

    // executing stargate query should return
    // an error defined in custom stargate keeper
    #[allow(deprecated)]
    let query: QueryRequest<Empty> = QueryRequest::Stargate {
        path: "test".to_string(),
        data: Default::default(),
    };
    assert_eq!(
        QUERY_MSG,
        app.wrap()
            .raw_query(to_json_vec(&query).unwrap().as_slice())
            .unwrap()
            .unwrap_err(),
    );
}

#[test]
fn building_app_with_accepting_stargate_should_work() {
    let app_builder = AppBuilder::default();
    let mut app = app_builder
        .with_stargate(StargateAcceptingModule::new())
        .build(no_init);

    // prepare user addresses
    let sender_addr = app.api().addr_make("sender");

    #[allow(deprecated)]
    let msg = CosmosMsg::Stargate {
        type_url: "test".to_string(),
        value: Default::default(),
    };
    app.execute(sender_addr, msg).unwrap();

    #[allow(deprecated)]
    let request = QueryRequest::Stargate {
        path: "test".to_string(),
        data: Default::default(),
    };
    let _: Empty = app.wrap().query(&request).unwrap();
}

#[test]
fn building_app_with_failing_stargate_should_work() {
    let app_builder = AppBuilder::default();
    let mut app = app_builder
        .with_stargate(StargateFailingModule::new())
        .build(no_init);

    // prepare user addresses
    let sender_addr = app.api().addr_make("sender");

    #[allow(deprecated)]
    let msg = CosmosMsg::Stargate {
        type_url: "test".to_string(),
        value: Default::default(),
    };
    app.execute(sender_addr, msg).unwrap_err();

    #[allow(deprecated)]
    let request = QueryRequest::Stargate {
        path: "test".to_string(),
        data: Default::default(),
    };
    app.wrap().query::<Empty>(&request).unwrap_err();
}
