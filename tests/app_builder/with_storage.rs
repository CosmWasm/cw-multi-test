use cosmwasm_std::testing::MockStorage;
use cw_multi_test::AppBuilder;

#[test]
fn building_app_with_custom_storage_should_work() {
    let mut initialized = false;
    let app_builder = AppBuilder::default();
    let _ = app_builder
        .with_storage(MockStorage::default())
        .build(|_, _, _| {
            initialized = true;
        });
    assert!(initialized);
}
