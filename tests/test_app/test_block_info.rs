use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{BlockInfo, Timestamp};
use cw_multi_test::{next_block, App};

#[test]
fn default_block_info_should_work() {
    let env = mock_env();
    let app = App::default();
    let block = app.block_info();
    assert_eq!(env.block.chain_id, block.chain_id);
    assert_eq!(env.block.height, block.height);
    assert_eq!(env.block.time, block.time);
}

#[test]
fn setting_block_info_should_work() {
    let initial_block = BlockInfo {
        chain_id: "mainnet-fermentation".to_string(),
        height: 273_094,
        time: Timestamp::default().plus_days(366),
    };
    let mut app = App::default();
    app.set_block(initial_block.clone());
    let block = app.block_info();
    assert_eq!(initial_block.chain_id, block.chain_id);
    assert_eq!(initial_block.height, block.height);
    assert_eq!(initial_block.time, block.time);
}

#[test]
fn incrementing_block_info_should_work() {
    let env = mock_env();
    let mut app = App::default();
    app.update_block(next_block);
    let block = app.block_info();
    assert_eq!(env.block.chain_id, block.chain_id);
    assert_eq!(env.block.height + 1, block.height);
    assert_eq!(env.block.time.plus_seconds(5), block.time);
}
