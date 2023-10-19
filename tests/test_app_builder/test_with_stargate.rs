use crate::test_app_builder::MyKeeper;
use cosmwasm_std::{to_vec, Addr, Empty, QueryRequest};
use cw_multi_test::{AppBuilder, Executor, Stargate, StargateMsg, StargateQuery};

type MyStargateKeeper = MyKeeper<StargateMsg, StargateQuery, Empty>;

impl Stargate for MyStargateKeeper {}

const EXECUTE_MSG: &str = "stargate execute called";
const QUERY_MSG: &str = "stargate query called";

#[test]
fn building_app_with_custom_stargate_should_work() {
    // build custom stargate keeper, stargate keeper has no sudo messages
    let stargate_keeper = MyStargateKeeper::new(EXECUTE_MSG, QUERY_MSG, "");

    // build the application with custom stargate keeper
    let app_builder = AppBuilder::default();
    let mut app = app_builder
        .with_stargate(stargate_keeper)
        .build(|_, _, _| {});

    // executing stargate message should return an error defined in custom keeper
    assert_eq!(
        app.execute(
            Addr::unchecked("sender"),
            StargateMsg {
                type_url: "test".to_string(),
                value: Default::default()
            }
            .into(),
        )
        .unwrap_err()
        .to_string(),
        EXECUTE_MSG,
    );

    let query: QueryRequest<Empty> = StargateQuery {
        path: "test".to_string(),
        data: Default::default(),
    }
    .into();

    // executing stargate query should return an error defined in custom keeper
    assert_eq!(
        app.wrap()
            .raw_query(to_vec(&query).unwrap().as_slice())
            .unwrap()
            .unwrap_err(),
        QUERY_MSG
    );
}
