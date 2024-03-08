use cosmwasm_std::testing::MockApi;
use cw_multi_test::IntoAddr;

#[test]
fn conversion_with_default_prefix_should_work() {
    assert_eq!(
        MockApi::default().addr_make("creator").as_str(),
        "creator".into_addr().as_str(),
    );
}

#[test]
fn conversion_with_custom_prefix_should_work() {
    assert_eq!(
        MockApi::default()
            .with_prefix("juno")
            .addr_make("sender")
            .as_str(),
        "sender".into_addr_with_prefix("juno").as_str(),
    );
}
