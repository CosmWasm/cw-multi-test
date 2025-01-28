#[test]
fn default_initialization_should_work() {
    use cw_multi_test::{no_init, AppBuilder};

    let app = AppBuilder::default().build(no_init);

    let sender_addr = app.api().addr_make("sender");

    assert!(sender_addr.as_str().starts_with("cosmwasm1"));
}

#[test]
fn new_constructor_should_work() {
    use cw_multi_test::{no_init, AppBuilder};

    let app = AppBuilder::new().build(no_init);

    let sender_addr = app.api().addr_make("sender");

    assert!(sender_addr.as_str().starts_with("cosmwasm1"));
}
