use crate::test_app_builder::{MyKeeper, NO_MESSAGE};
use cosmwasm_std::{DistributionMsg, Empty};
use cw_multi_test::{no_init, AppBuilder, Distribution, Executor};

type MyDistributionKeeper = MyKeeper<DistributionMsg, Empty, Empty>;

impl Distribution for MyDistributionKeeper {}

const EXECUTE_MSG: &str = "distribution execute called";

#[test]
fn building_app_with_custom_distribution_should_work() {
    // build custom distribution keeper
    // which has no query or sudo messages
    let distribution_keeper = MyDistributionKeeper::new(EXECUTE_MSG, NO_MESSAGE, NO_MESSAGE);

    // build the application with custom distribution keeper
    let app_builder = AppBuilder::default();
    let mut app = app_builder
        .with_distribution(distribution_keeper)
        .build(no_init);

    // prepare additional input data
    let recipient = app.api().addr_make("recipient");

    // executing distribution message should return an error defined in custom keeper
    assert_eq!(
        EXECUTE_MSG,
        app.execute(
            app.api().addr_make("sender"),
            DistributionMsg::SetWithdrawAddress {
                address: recipient.into(),
            }
            .into(),
        )
        .unwrap_err()
        .to_string()
    );
}
