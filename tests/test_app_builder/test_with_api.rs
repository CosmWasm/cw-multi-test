use crate::test_api::MockApiBech32;
use cosmwasm_std::{Addr, Api, CanonicalAddr, HexBinary};
use cw_multi_test::AppBuilder;

#[test]
fn building_app_with_custom_api_should_work() {
    // prepare test data
    let human = "juno1h34lmpywh4upnjdg90cjf4j70aee6z8qqfspugamjp42e4q28kqsksmtyp";
    let hex = "bc6bfd848ebd7819c9a82bf124d65e7f739d08e002601e23bb906aacd40a3d81";

    // create application with custom api that implements
    // Bech32 address encoding with 'juno' prefix
    let app = AppBuilder::default()
        .with_api(MockApiBech32::new("juno"))
        .build(|_, _, _| {});

    // check address validation function
    assert_eq!(
        app.api().addr_validate(human).unwrap(),
        Addr::unchecked(human)
    );

    // check if address can be canonicalized
    assert_eq!(
        app.api().addr_canonicalize(human).unwrap(),
        CanonicalAddr::from(HexBinary::from_hex(hex).unwrap())
    );

    // check if address can be humanized
    assert_eq!(
        app.api()
            .addr_humanize(&app.api().addr_canonicalize(human).unwrap())
            .unwrap(),
        Addr::unchecked(human)
    );

    // check extension function for creating Bech32 encoded addresses
    assert_eq!(app.api().addr_make("creator"), Addr::unchecked(human));
}
