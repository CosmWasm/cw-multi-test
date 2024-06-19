use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{Coin, Uint128};
use cw_multi_test::AppBuilder;

const USER: &str = "USER";
const NATIVE_DENOM: &str = "NativeDenom";
const AMOUNT: u128 = 100;

#[test]
fn initializing_balance_should_work() {
    let app = AppBuilder::new().build(|router, api, storage| {
        let _ = api;
        router
            .bank
            .init_balance(
                storage,
                &MockApi::default().addr_make(USER),
                vec![Coin {
                    denom: NATIVE_DENOM.to_string(),
                    amount: Uint128::new(100),
                }],
            )
            .unwrap();
    });
    let api = app.api();
    let user_addr = api.addr_make(USER);
    let balances = app.wrap().query_all_balances(user_addr).unwrap();
    assert_eq!(1, balances.len());
    assert_eq!(
        format!("{}{}", AMOUNT, NATIVE_DENOM),
        balances[0].to_string()
    );
}
