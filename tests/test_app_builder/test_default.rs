use cw_multi_test::IntoAddr;

#[test]
fn default_should_work() {
    use cw_multi_test::{no_init, AppBuilder};

    let app = AppBuilder::default().build(no_init);

    let sender_addr = app.api().addr_make("sender");

    assert!(sender_addr.as_str().starts_with("cosmwasm1"));
}

#[test]
fn default_with_initialization_should_work() {
    use cosmwasm_std::coin;
    use cw_multi_test::AppBuilder;

    let my_address = "me".into_addr();
    let my_funds = vec![coin(23, "ATOM"), coin(18, "FLOCK")];

    let app = AppBuilder::default().build(|router, _api, storage| {
        router
            .bank
            .init_balance(storage, &my_address, my_funds)
            .unwrap();
    });

    assert_eq!(
        "23ATOM",
        app.wrap()
            .query_balance(my_address, "ATOM")
            .unwrap()
            .to_string()
    );
}

#[test]
fn new_should_work() {
    use cw_multi_test::{no_init, AppBuilder};

    let app = AppBuilder::new().build(no_init);

    let sender_addr = app.api().addr_make("sender");

    assert!(sender_addr.as_str().starts_with("cosmwasm1"));
}
