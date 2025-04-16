use cosmwasm_std::testing::mock_env;
use cosmwasm_std::Empty;
use cw_multi_test::{AppBuilder, Module};

#[test]
fn distribution_sudo_should_fail() {
    let block = mock_env().block;
    let _ = AppBuilder::default().build(|router, api, storage| {
        // Calling sudo on distribution should fail.
        assert_eq!(
            "Something went wrong - distribution doesn't have sudo messages",
            router
                .distribution
                .sudo(api, storage, router, &block, Empty {})
                .unwrap_err()
                .to_string()
        );
    });
}
