use crate::test_app_builder::{MyKeeper, NO_MESSAGE};
use cosmwasm_std::{to_json_vec, Addr, AnyMsg, CosmosMsg, Empty, GrpcQuery, QueryRequest};
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

    // executing stargate message should return
    // an error defined in custom stargate keeper
    assert_eq!(
        app.execute(
            Addr::unchecked("sender"),
            CosmosMsg::Any(AnyMsg {
                type_url: "test".to_string(),
                value: Default::default()
            }),
        )
        .unwrap_err()
        .to_string(),
        EXECUTE_MSG,
    );

    // executing stargate query should return
    // an error defined in custom stargate keeper
    let query: QueryRequest<Empty> = QueryRequest::Grpc(GrpcQuery {
        path: "test".to_string(),
        data: Default::default(),
    });
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
        .with_stargate(StargateAcceptingModule::new())
        .build(no_init);

    app.execute(
        Addr::unchecked("sender"),
        CosmosMsg::Any(AnyMsg {
            type_url: "test".to_string(),
            value: Default::default(),
        }),
    )
    .unwrap();

    let _ = app
        .wrap()
        .query::<Empty>(&QueryRequest::Grpc(GrpcQuery {
            path: "test".to_string(),
            data: Default::default(),
        }))
        .is_ok();
}

#[test]
fn building_app_with_failing_stargate_should_work() {
    let app_builder = AppBuilder::default();
    let mut app = app_builder
        .with_stargate(StargateFailingModule::new())
        .build(no_init);

    app.execute(
        Addr::unchecked("sender"),
        CosmosMsg::Any(AnyMsg {
            type_url: "test".to_string(),
            value: Default::default(),
        }),
    )
    .unwrap_err();

    let _ = app
        .wrap()
        .query::<Empty>(&QueryRequest::Grpc(GrpcQuery {
            path: "test".to_string(),
            data: Default::default(),
        }))
        .unwrap_err();
}
