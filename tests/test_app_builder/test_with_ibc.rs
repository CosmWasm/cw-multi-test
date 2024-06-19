use crate::test_app_builder::{MyKeeper, NO_MESSAGE};
use cosmwasm_std::{Empty, IbcMsg, IbcQuery, QueryRequest};
use cw_multi_test::{no_init, AppBuilder, Executor, Ibc};

type MyIbcKeeper = MyKeeper<IbcMsg, IbcQuery, Empty>;

impl Ibc for MyIbcKeeper {}

const EXECUTE_MSG: &str = "ibc execute called";
const QUERY_MSG: &str = "ibc query called";

#[test]
fn building_app_with_custom_ibc_should_work() {
    // build custom ibc keeper (no sudo handling for ibc)
    let ibc_keeper = MyIbcKeeper::new(EXECUTE_MSG, QUERY_MSG, NO_MESSAGE);

    // build the application with custom ibc keeper
    let mut app = AppBuilder::default().with_ibc(ibc_keeper).build(no_init);

    // prepare user addresses
    let sender_addr = app.api().addr_make("sender");

    // executing ibc message should return an error defined in custom keeper
    assert_eq!(
        EXECUTE_MSG,
        app.execute(
            sender_addr,
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
