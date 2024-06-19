use cosmwasm_std::{BlockInfo, Timestamp};
use cw_multi_test::AppBuilder;

#[test]
fn building_app_with_custom_block_should_work() {
    // prepare additional test data
    let block_info = BlockInfo {
        height: 20,
        time: Timestamp::from_nanos(1_571_797_419_879_305_544),
        chain_id: "my-testnet".to_string(),
    };

    // build the application with custom block
    let app_builder = AppBuilder::default();
    let app = app_builder.with_block(block_info.clone()).build_no_init();

    // calling block_info should return the same block used during initialization
    assert_eq!(block_info, app.block_info());
}
