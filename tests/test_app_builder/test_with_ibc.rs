use crate::test_app_builder::{MyKeeper, NO_MESSAGE};
use cosmwasm_std::{Addr, IbcMsg, IbcQuery, QueryRequest};
use cw_multi_test::{
    ibc::{types::MockIbcQuery, IbcPacketRelayingMsg},
    AppBuilder, Executor, Ibc,
};

type MyIbcKeeper = MyKeeper<IbcMsg, MockIbcQuery, IbcPacketRelayingMsg>;

impl Ibc for MyIbcKeeper {}

const EXECUTE_MSG: &str = "ibc execute called";
const QUERY_MSG: &str = "ibc query called";

#[test]
fn building_app_with_custom_ibc_should_work() {
    // build custom ibc keeper (no sudo handling for ibc)
    let ibc_keeper = MyIbcKeeper::new(EXECUTE_MSG, QUERY_MSG, NO_MESSAGE);

    // build the application with custom ibc keeper
    let app_builder = AppBuilder::default();
    let mut app = app_builder.with_ibc(ibc_keeper).build(|_, _, _| {});

    // executing ibc message should return an error defined in custom keeper
    assert_eq!(
        EXECUTE_MSG,
        app.execute(
            Addr::unchecked("sender"),
            IbcMsg::CloseChannel {
                channel_id: "my-channel".to_string()
            }
            .into(),
        )
        .unwrap_err()
        .to_string()
    );

    // executing ibc query should return an error defined in custom keeper
    assert_eq!(
        format!("Generic error: Querier contract error: {}", QUERY_MSG),
        app.wrap()
            .query::<IbcQuery>(&QueryRequest::Ibc(IbcQuery::ListChannels {
                port_id: Some("my-port".to_string())
            }))
            .unwrap_err()
            .to_string()
    );
}
