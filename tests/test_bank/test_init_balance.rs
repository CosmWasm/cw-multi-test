use cosmwasm_std::{Coin, Uint128};
use cw_multi_test::AppBuilder;

const USER: &str = "user";
const DENOM: &str = "denom";
const AMOUNT: u128 = 100;

#[test]
fn initializing_balance_should_work() {
    let app = AppBuilder::new().build(|router, api, storage| {
        router
            .bank
            .init_balance(
                storage,
                &api.addr_make(USER),
                vec![Coin {
                    denom: DENOM.to_string(),
                    amount: Uint128::new(AMOUNT),
                }],
            )
            .unwrap();
    });
    let balances = app
        .wrap()
        .query_all_balances(app.api().addr_make(USER))
        .unwrap();
    assert_eq!(1, balances.len());
    assert_eq!(AMOUNT, balances[0].amount.u128());
    assert_eq!(DENOM, balances[0].denom);
}
