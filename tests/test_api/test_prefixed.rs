use super::*;
use cosmwasm_std::CanonicalAddr;
use cw_multi_test::MockApiBech32;

const HUMAN_ADDRESS: &str = "juno1h34lmpywh4upnjdg90cjf4j70aee6z8qqfspugamjp42e4q28kqsksmtyp";

fn api_prefix(prefix: &'static str) -> MockApiBech32 {
    MockApiBech32::new(prefix)
}

fn api_juno() -> MockApiBech32 {
    api_prefix("juno")
}

fn api_osmo() -> MockApiBech32 {
    api_prefix("osmo")
}

#[test]
fn new_api_bech32_should_work() {
    let addr = api_juno().addr_make("creator");
    assert_eq!(HUMAN_ADDRESS, addr.as_str(),);
}

#[test]
fn address_validate_should_work() {
    assert_eq!(
        api_juno().addr_validate(HUMAN_ADDRESS).unwrap().as_str(),
        HUMAN_ADDRESS
    )
}

#[test]
fn address_validate_invalid_address() {
    api_juno().addr_validate("creator").unwrap_err();
}

#[test]
fn addr_validate_invalid_prefix() {
    api_juno()
        .addr_validate(api_osmo().addr_make("creator").as_str())
        .unwrap_err();
}

#[test]
fn address_canonicalize_humanize_should_work() {
    let api = api_juno();
    assert_eq!(
        api.addr_humanize(&api.addr_canonicalize(HUMAN_ADDRESS).unwrap())
            .unwrap()
            .as_str(),
        HUMAN_ADDRESS
    );
}

#[test]
fn address_humanize_prefix_too_long() {
    api_prefix(
        "juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_",
    )
    .addr_humanize(&CanonicalAddr::from([1, 2, 3, 4, 5]))
    .unwrap_err();
}

#[test]
fn debug_should_not_panic() {
    assert_debug_does_not_panic(&api_juno());
}

#[test]
#[should_panic(
    expected = "Generating address failed with reason: hrp is too long, found 85 characters, must be <= 126"
)]
fn address_make_prefix_too_long() {
    api_prefix(
        "juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_",
    )
    .addr_make("creator");
}

#[test]
fn secp256k1_verify_works() {
    assert_secp256k1_verify_works(&api_juno());
}

#[test]
fn secp256k1_recover_pubkey_works() {
    assert_secp256k1_recover_pubkey_works(&api_juno());
}

#[test]
fn ed25519_verify_works() {
    assert_ed25519_verify_works(&api_juno());
}

#[test]
fn ed25519_batch_verify_works() {
    assert_ed25519_batch_verify_works(&api_juno());
}
