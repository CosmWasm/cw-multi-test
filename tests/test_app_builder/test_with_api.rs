#[test]
fn building_app_with_customized_api_should_work() {
    use cosmwasm_std::testing::MockApi;
    use cosmwasm_std::{Api, CanonicalAddr, HexBinary};
    use cw_multi_test::{no_init, AppBuilder};

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

#[test]
fn default_api_should_work() {
    use cw_multi_test::{no_init, AppBuilder};

    // Create the chain with default Api implementation.
    let app = AppBuilder::default().build(no_init);

    // Create the address using default Api.
    let sender_addr = app.api().addr_make("sender");

    // Default Api generates Bech32 addresses with prefix 'cosmwasm'.
    assert_eq!(
        "cosmwasm1pgm8hyk0pvphmlvfjc8wsvk4daluz5tgrw6pu5mfpemk74uxnx9qlm3aqg",
        sender_addr.as_str()
    );
}

#[test]
fn customized_api_should_work() {
    use cosmwasm_std::testing::MockApi;
    use cw_multi_test::{no_init, AppBuilder};

    // Create the chain with customized default Api implementation.
    let app = AppBuilder::default()
        .with_api(MockApi::default().with_prefix("osmo"))
        .build(no_init);

    // Create the address using customized Api.
    let sender_addr = app.api().addr_make("sender");

    // This customized Api generates Bech32 addresses with prefix 'osmo'.
    assert_eq!(
        "osmo1pgm8hyk0pvphmlvfjc8wsvk4daluz5tgrw6pu5mfpemk74uxnx9qcrt3u2",
        sender_addr.as_str()
    );
}

#[test]
fn bech32_api_should_work() {
    use cw_multi_test::MockApiBech32;
    use cw_multi_test::{no_init, AppBuilder};

    // Create the chain with Bech32 Api implementation.
    let app = AppBuilder::default()
        .with_api(MockApiBech32::new("juno"))
        .build(no_init);

    // Create the address using Bech32 Api.
    let sender_addr = app.api().addr_make("sender");

    // This Api generates Bech32 addresses with prefix 'juno'.
    assert_eq!(
        "juno1pgm8hyk0pvphmlvfjc8wsvk4daluz5tgrw6pu5mfpemk74uxnx9qwm56ug",
        sender_addr.as_str()
    );
}

#[test]
fn bech32m_api_should_work() {
    use cw_multi_test::MockApiBech32m;
    use cw_multi_test::{no_init, AppBuilder};

    // Create the chain with Bech32m Api implementation.
    let app = AppBuilder::default()
        .with_api(MockApiBech32m::new("juno"))
        .build(no_init);

    // Create the address using Bech32m Api.
    let sender_addr = app.api().addr_make("sender");

    // This Api generates Bech32m addresses with prefix 'juno'.
    assert_eq!(
        "juno1pgm8hyk0pvphmlvfjc8wsvk4daluz5tgrw6pu5mfpemk74uxnx9qm8yke2",
        sender_addr.as_str()
    );
}
