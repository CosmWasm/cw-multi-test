use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{Api, CanonicalAddr, HexBinary};
use cw_multi_test::{no_init, AppBuilder};

#[test]
fn building_app_with_customized_api_should_work() {
    // Prepare test data.
    let human = "juno1h34lmpywh4upnjdg90cjf4j70aee6z8qqfspugamjp42e4q28kqsksmtyp";
    let hex = "bc6bfd848ebd7819c9a82bf124d65e7f739d08e002601e23bb906aacd40a3d81";

    // Create the chain with customized Api that implements
    // Bech32 address encoding with 'juno' prefix.
    let app = AppBuilder::default()
        .with_api(MockApi::default().with_prefix("juno"))
        .build(no_init);

    // Check the address validation function.
    assert_eq!(human, app.api().addr_validate(human).unwrap().as_str());

    // Check if the address can be canonicalized.
    assert_eq!(
        app.api().addr_canonicalize(human).unwrap(),
        CanonicalAddr::from(HexBinary::from_hex(hex).unwrap())
    );

    // Check if the address can be humanized.
    assert_eq!(
        human,
        app.api()
            .addr_humanize(&app.api().addr_canonicalize(human).unwrap())
            .unwrap()
            .as_str(),
    );

    // Check the extension function for creating Bech32 encoded addresses.
    assert_eq!(human, app.api().addr_make("creator").as_str());
}
