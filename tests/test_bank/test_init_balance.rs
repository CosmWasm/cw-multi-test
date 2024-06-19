use cosmwasm_std::{Coin, CustomMsg, CustomQuery, Uint128};
use cw_multi_test::{custom_app, App, AppBuilder, BasicApp};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const USER: &str = "user";
const DENOM: &str = "denom";
const AMOUNT: u128 = 100;

fn assert_balance(coins: Vec<Coin>) {
    assert_eq!(1, coins.len());
    assert_eq!(AMOUNT, coins[0].amount.u128());
    assert_eq!(DENOM, coins[0].denom);
}

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
    assert_balance(
        app.wrap()
            .query_all_balances(app.api().addr_make(USER))
            .unwrap(),
    );
}

#[test]
fn initializing_balance_without_builder_should_work() {
    let app = App::new(|router, api, storage| {
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
    assert_balance(
        app.wrap()
            .query_all_balances(app.api().addr_make(USER))
            .unwrap(),
    );
}

#[test]
fn initializing_balance_custom_app_should_work() {
    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
    #[serde(rename = "snake_case")]
    pub enum CustomHelperMsg {
        HelperMsg,
    }
    impl CustomMsg for CustomHelperMsg {}

    #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
    #[serde(rename = "snake_case")]
    pub enum CustomHelperQuery {
        HelperQuery,
    }
    impl CustomQuery for CustomHelperQuery {}

    let app: BasicApp<CustomHelperMsg, CustomHelperQuery> = custom_app(|router, api, storage| {
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
    assert_balance(
        app.wrap()
            .query_all_balances(app.api().addr_make(USER))
            .unwrap(),
    );
}

#[test]
fn initializing_balance_later_should_work() {
    let mut app = App::default();
    app.init_modules(|router, api, storage| {
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
    assert_balance(
        app.wrap()
            .query_all_balances(app.api().addr_make(USER))
            .unwrap(),
    );
}
