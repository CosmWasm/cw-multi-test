use cw_multi_test::{AppBuilder, BankKeeper};

#[test]
fn building_app_with_custom_bank_should_work() {
    let mut initialized = false;
    let app_builder = AppBuilder::default();
    let _ = app_builder
        .with_bank(BankKeeper::default())
        .build(|_, _, _| {
            initialized = true;
        });
    assert!(initialized);
}
