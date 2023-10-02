use cosmwasm_std::{BlockInfo, Timestamp};
use cw_multi_test::AppBuilder;

#[test]
fn building_app_with_custom_block_should_work() {
    let app_builder = AppBuilder::default();
    let _ = app_builder
        .with_block(BlockInfo {
            height: 20,
            time: Timestamp::from_nanos(1_571_797_419_879_305_544),
            chain_id: "my-testnet".to_string(),
        })
        .build(|_, _, _| {});
}
