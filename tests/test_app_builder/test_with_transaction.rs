use cosmwasm_std::TransactionInfo;
use cw_multi_test::{no_init, AppBuilder};

#[test]
fn building_app_with_custom_transaction_info_should_work() {
    // prepare custom transaction info
    let transaction_info = TransactionInfo { index: 21 };

    // build the application with custom transaction info
    let app_builder = AppBuilder::default();
    let app = app_builder
        .with_transaction(transaction_info)
        .build(no_init);

    // index should be the same value as provided during initialization
    assert_eq!(21, app.transaction_info().index);
}
