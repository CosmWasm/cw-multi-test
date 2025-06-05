use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, CustomMsg, CustomQuery, Uint256};
use cw_multi_test::{custom_app, App, AppBuilder, BasicApp};

const USER: &str = "user";
const DENOM: &str = "denom";
const AMOUNT: u128 = 100;

fn assert_balance(coin: Coin) {
    assert_eq!(Uint256::new(AMOUNT), coin.amount);
    assert_eq!(DENOM, coin.denom);
}

fn coins() -> Vec<Coin> {
    vec![Coin {
        denom: DENOM.to_string(),
        amount: Uint256::new(AMOUNT),
    }]
}

#[test]
fn initializing_balance_should_work() {
    let app = AppBuilder::new().build(|router, api, storage| {
        router
            .bank
            .init_balance(storage, &api.addr_make(USER), coins())
            .unwrap();
    });
    #[allow(deprecated)]
    assert_balance(
        app.wrap()
            .query_balance(app.api().addr_make(USER), DENOM)
            .unwrap(),
    );
}

#[test]
fn initializing_balance_without_builder_should_work() {
    let app = App::new(|router, api, storage| {
        router
            .bank
            .init_balance(storage, &api.addr_make(USER), coins())
            .unwrap();
    });
    #[allow(deprecated)]
    assert_balance(
        app.wrap()
            .query_balance(app.api().addr_make(USER), DENOM)
            .unwrap(),
    );
}

#[test]
fn initializing_balance_custom_app_should_work() {
    #[cw_serde]
    pub enum CustomHelperMsg {
        HelperMsg,
    }
    impl CustomMsg for CustomHelperMsg {}

    #[cw_serde]
    pub enum CustomHelperQuery {
        HelperQuery,
    }
    impl CustomQuery for CustomHelperQuery {}

    let app: BasicApp<CustomHelperMsg, CustomHelperQuery> = custom_app(|router, api, storage| {
        router
            .bank
            .init_balance(storage, &api.addr_make(USER), coins())
            .unwrap();
    });
    #[allow(deprecated)]
    assert_balance(
        app.wrap()
            .query_balance(app.api().addr_make(USER), DENOM)
            .unwrap(),
    );
}

#[test]
fn initializing_balance_later_should_work() {
    let mut app = App::default();
    app.init_modules(|router, api, storage| {
        router
            .bank
            .init_balance(storage, &api.addr_make(USER), coins())
            .unwrap();
    });
    #[allow(deprecated)]
    assert_balance(
        app.wrap()
            .query_balance(app.api().addr_make(USER), DENOM)
            .unwrap(),
    );
}
