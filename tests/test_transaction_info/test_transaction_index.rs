use crate::test_contracts::counter;
use cosmwasm_std::{BlockInfo, Empty, Timestamp};
use cw_multi_test::{App, Executor};

#[test]
fn default_transaction_index_should_be_zero() {
    let app = App::default();
    assert_eq!(0, app.transaction_info().index);
}

#[test]
fn instantiate_should_increment_transaction_index() {
    let mut app = App::default();
    let sender_addr = app.api().addr_make("sender");
    let code_id = app.store_code(counter::contract());
    app.instantiate_contract(code_id, sender_addr, &Empty {}, &[], "counter", None)
        .unwrap();
    assert_eq!(1, app.transaction_info().index);
}

#[test]
fn execute_should_increment_transaction_index() {
    let mut app = App::default();
    let sender_addr = app.api().addr_make("sender");
    let code_id = app.store_code(counter::contract());
    let contract_addr = app
        .instantiate_contract(
            code_id,
            sender_addr.clone(),
            &Empty {},
            &[],
            "counter",
            None,
        )
        .unwrap();
    assert_eq!(1, app.transaction_info().index);
    app.execute_contract(sender_addr, contract_addr, &Empty {}, &[])
        .unwrap();
    assert_eq!(2, app.transaction_info().index);
}

#[test]
fn setting_block_should_reset_transaction_index() {
    let mut app = App::default();
    let sender_addr = app.api().addr_make("sender");
    let code_id = app.store_code(counter::contract());
    let contract_addr = app
        .instantiate_contract(
            code_id,
            sender_addr.clone(),
            &Empty {},
            &[],
            "counter",
            None,
        )
        .unwrap();
    assert_eq!(1, app.transaction_info().index);
    app.execute_contract(sender_addr.clone(), contract_addr.clone(), &Empty {}, &[])
        .unwrap();
    assert_eq!(2, app.transaction_info().index);
    // prepare custom block properties
    let block = BlockInfo {
        height: 20,
        time: Timestamp::from_nanos(1_571_797_419_879_305_544),
        chain_id: "my-testnet".to_string(),
    };
    app.set_block(block);
    app.execute_contract(sender_addr, contract_addr, &Empty {}, &[])
        .unwrap();
    assert_eq!(1, app.transaction_info().index);
}

#[test]
fn updating_block_should_reset_transaction_index() {
    let mut app = App::default();
    let sender_addr = app.api().addr_make("sender");
    let code_id = app.store_code(counter::contract());
    let contract_addr = app
        .instantiate_contract(
            code_id,
            sender_addr.clone(),
            &Empty {},
            &[],
            "counter",
            None,
        )
        .unwrap();
    assert_eq!(1, app.transaction_info().index);
    app.execute_contract(sender_addr.clone(), contract_addr.clone(), &Empty {}, &[])
        .unwrap();
    assert_eq!(2, app.transaction_info().index);
    app.update_block(|block| {
        block.height += 1;
        block.time = block.time.plus_seconds(60);
    });
    app.execute_contract(sender_addr, contract_addr, &Empty {}, &[])
        .unwrap();
    assert_eq!(1, app.transaction_info().index);
}
