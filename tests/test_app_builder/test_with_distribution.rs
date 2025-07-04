use crate::test_app_builder::MyKeeper;
use cosmwasm_std::{DistributionMsg, DistributionQuery, Empty};
use cw_multi_test::{no_init, AppBuilder, Distribution, Executor};

type MyDistributionKeeper = MyKeeper<DistributionMsg, DistributionQuery, Empty>;

impl Distribution for MyDistributionKeeper {}

const EXECUTE_MSG: &str = "distribution execute called";

#[test]
fn building_app_with_custom_distribution_should_work() {
    // build custom distribution keeper
    // which has no query or sudo messages
    let distribution_keeper = MyDistributionKeeper::new(EXECUTE_MSG, "", "");

    // build the application with custom distribution keeper
    let app_builder = AppBuilder::default();
    let mut app = app_builder
        .with_distribution(distribution_keeper)
        .build(no_init);

    // prepare addresses
    let recipient_addr = app.api().addr_make("recipient");
    let sender_addr = app.api().addr_make("sender");

    // executing distribution message should return an error defined in custom keeper
    assert_eq!(
        format!("kind: Other, error: {EXECUTE_MSG}"),
        app.execute(
            sender_addr,
            DistributionMsg::SetWithdrawAddress {
                address: recipient_addr.into(),
            }
            .into(),
        )
        .unwrap_err()
        .to_string()
    );
}
