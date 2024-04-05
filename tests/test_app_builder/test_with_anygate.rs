use crate::test_app_builder::{MyKeeper, NO_MESSAGE};
use cosmwasm_std::{to_json_vec, AnyMsg, CosmosMsg, Empty, GrpcQuery, QueryRequest};
use cw_multi_test::{
    no_init, Anygate, AnygateAcceptingModule, AnygateFailingModule, AppBuilder, Executor,
};

type MyAnygateKeeper = MyKeeper<AnyMsg, GrpcQuery, Empty>;

impl Anygate for MyAnygateKeeper {}

const EXECUTE_MSG: &str = "anygate execute called";
const QUERY_MSG: &str = "anygate query called";

#[test]
fn building_app_with_custom_anygate_should_work() {
    // build custom anygate keeper
    let anygate_keeper = MyAnygateKeeper::new(EXECUTE_MSG, QUERY_MSG, NO_MESSAGE);

    // build the application with custom anygate keeper
    let app_builder = AppBuilder::default();
    let mut app = app_builder.with_anygate(anygate_keeper).build(no_init);

    // prepare user addresses
    let sender_addr = app.api().addr_make("sender");

    // executing anygate message should return
    // an error defined in custom anygate keeper
    assert_eq!(
        app.execute(
            sender_addr,
            CosmosMsg::Any(AnyMsg {
                type_url: "test".to_string(),
                value: Default::default()
            }),
        )
        .unwrap_err()
        .to_string(),
        EXECUTE_MSG,
    );

    // executing anygate query should return
    // an error defined in custom anygate keeper
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
fn building_app_with_accepting_anygate_should_work() {
    let app_builder = AppBuilder::default();
    let mut app = app_builder
        .with_anygate(AnygateAcceptingModule::new())
        .build(no_init);

    // prepare user addresses
    let sender_addr = app.api().addr_make("sender");

    app.execute(
        sender_addr,
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
fn building_app_with_failing_anygate_should_work() {
    let app_builder = AppBuilder::default();
    let mut app = app_builder
        .with_anygate(AnygateFailingModule::new())
        .build(no_init);

    // prepare user addresses
    let sender_addr = app.api().addr_make("sender");

    app.execute(
        sender_addr,
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
