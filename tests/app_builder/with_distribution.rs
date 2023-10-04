use cw_multi_test::{AppBuilder, DistributionKeeper};

#[test]
fn building_app_with_custom_distribution_should_work() {
    let mut initialized = false;
    let app_builder = AppBuilder::default();
    let _ = app_builder
        .with_distribution(DistributionKeeper::default())
        .build(|_, _, _| {
            initialized = true;
        });
    assert!(initialized);
}
