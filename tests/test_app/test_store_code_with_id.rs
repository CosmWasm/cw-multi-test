use crate::test_contracts::counter;
use cw_multi_test::App;

#[test]
fn storing_code_with_custom_identifier_should_work() {
    let mut app = App::default();
    let creator = app.api().addr_make("prometheus");
    assert_eq!(
        10,
        app.store_code_with_id(creator.clone(), 10, counter::contract())
            .unwrap()
    );
    assert_eq!(
        u64::MAX,
        app.store_code_with_id(creator, u64::MAX, counter::contract())
            .unwrap()
    );
}

#[test]
fn zero_code_id_is_not_allowed() {
    let mut app = App::default();
    let creator = app.api().addr_make("prometheus");
    app.store_code_with_id(creator, 0, counter::contract())
        .unwrap_err();
}

#[test]
fn storing_code_with_consecutive_identifiers() {
    let mut app = App::default();
    let creator = app.api().addr_make("prometheus");
    assert_eq!(
        11,
        app.store_code_with_id(creator, 11, counter::contract())
            .unwrap()
    );
    for i in 12..=20 {
        assert_eq!(i, app.store_code(counter::contract()));
    }
}

#[test]
fn storing_with_the_same_id_is_not_allowed() {
    let mut app = App::default();
    let creator = app.api().addr_make("prometheus");
    let code_id = 2056;
    assert_eq!(
        code_id,
        app.store_code_with_id(creator.clone(), code_id, counter::contract())
            .unwrap()
    );
    app.store_code_with_id(creator, code_id, counter::contract())
        .unwrap_err();
}

#[test]
#[should_panic(expected = "called `Result::unwrap()` on an `Err` value: code id: invalid")]
fn no_more_identifiers_available() {
    let mut app = App::default();
    let creator = app.api().addr_make("prometheus");
    assert_eq!(
        u64::MAX,
        app.store_code_with_id(creator, u64::MAX, counter::contract())
            .unwrap()
    );
    app.store_code(counter::contract());
}
