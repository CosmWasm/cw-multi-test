use super::*;
use cosmwasm_std::CanonicalAddr;
use cw_multi_test::{IntoBech32, IntoBech32m, MockApiBech32, MockApiBech32m};

const ADDR_JUNO: &str = "juno1h34lmpywh4upnjdg90cjf4j70aee6z8qqfspugamjp42e4q28kqsrvt8pr";
const ADDR_DEFAULT: &str = "cosmwasm1h34lmpywh4upnjdg90cjf4j70aee6z8qqfspugamjp42e4q28kqsjvwqar";

#[test]
fn new_api_bech32m_should_work() {
    assert_eq!(
        MockApiBech32m::new("juno").addr_make("creator").as_str(),
        ADDR_JUNO
    );
    assert_eq!(
        "creator".into_bech32m_with_prefix("juno").as_str(),
        ADDR_JUNO
    );
    assert_eq!("creator".into_bech32m().as_str(), ADDR_DEFAULT);
}

#[test]
fn api_bech32m_should_differ_from_bech32() {
    assert_ne!(
        MockApiBech32m::new("juno").addr_make("sender").as_str(),
        MockApiBech32::new("juno").addr_make("sender").as_str()
    );
    assert_ne!(
        "sender".into_bech32m_with_prefix("juno").as_str(),
        "sender".into_bech32_with_prefix("juno").as_str()
    );
    assert_ne!(
        "sender".into_bech32m().as_str(),
        "sender".into_bech32().as_str()
    );
}

#[test]
fn address_validate_should_work() {
    assert_eq!(
        MockApiBech32m::new("juno")
            .addr_validate(ADDR_JUNO)
            .unwrap()
            .as_str(),
        ADDR_JUNO
    )
}

#[test]
fn address_validate_invalid_address() {
    MockApiBech32m::new("juno")
        .addr_validate("creator")
        .unwrap_err();
}

#[test]
fn addr_validate_invalid_prefix() {
    MockApiBech32m::new("juno")
        .addr_validate(MockApiBech32m::new("osmosis").addr_make("creator").as_str())
        .unwrap_err();
}

#[test]
fn address_validate_invalid_variant() {
    MockApiBech32m::new("juno")
        .addr_validate(MockApiBech32::new("juno").addr_make("creator").as_str())
        .unwrap_err();
}

#[test]
fn address_canonicalize_humanize_should_work() {
    let api = MockApiBech32m::new("juno");
    assert_eq!(
        api.addr_humanize(&api.addr_canonicalize(ADDR_JUNO).unwrap())
            .unwrap()
            .as_str(),
        ADDR_JUNO
    );
}

#[test]
fn address_humanize_prefix_too_long() {
    assert_eq!(
        "Generic error: hrp is too long, found 85 characters, must be <= 126",
        MockApiBech32m::new(
            "juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_",
        )
        .addr_humanize(&CanonicalAddr::from([1, 2, 3, 4, 5]))
        .unwrap_err()
        .to_string()
    );
}

#[test]
fn address_humanize_canonical_too_long() {
    assert_eq!(
        "Generic error: Invalid canonical address",
        MockApiBech32m::new("juno")
            .addr_humanize(&CanonicalAddr::from([1; 1024]))
            .unwrap_err()
            .to_string()
    );
}

#[test]
fn debug_should_not_panic() {
    assert_debug_does_not_panic(&MockApiBech32m::new("juno"));
}

#[test]
#[should_panic(
    expected = "Generating address failed with reason: hrp is too long, found 85 characters, must be <= 126"
)]
fn address_make_prefix_too_long() {
    MockApiBech32m::new(
        "juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_juno_",
    )
    .addr_make("creator");
}

#[test]
fn bls12_381_aggregate_g1_works() {
    assert_bls12_381_aggregate_g1_works(&MockApiBech32m::new("juno"));
}

#[test]
fn bls12_381_aggregate_g2_works() {
    assert_bls12_381_aggregate_g2_works(&MockApiBech32m::new("juno"));
}

#[test]
fn bls12_381_pairing_equality_works() {
    assert_bls12_381_pairing_equality_works(&MockApiBech32m::new("juno"));
}

#[test]
fn bls12_381_hash_to_g1_works() {
    assert_bls12_381_hash_to_g1_works(&MockApiBech32m::new("juno"));
}

#[test]
fn bls12_381_hash_to_g2_works() {
    assert_bls12_381_hash_to_g2_works(&MockApiBech32m::new("juno"));
}

#[test]
fn secp256k1_verify_works() {
    assert_secp256k1_verify_works(&MockApiBech32m::new("juno"));
}

#[test]
fn secp256k1_recover_pubkey_works() {
    assert_secp256k1_recover_pubkey_works(&MockApiBech32m::new("juno"));
}

#[test]
fn secp256r1_verify_works() {
    assert_secp256r1_verify_works(&MockApiBech32m::new("juno"));
}

#[test]
fn secp256r1_recover_pubkey_works() {
    assert_secp256r1_recover_pubkey_works(&MockApiBech32m::new("juno"));
}

#[test]
fn ed25519_verify_works() {
    assert_ed25519_verify_works(&MockApiBech32m::new("juno"));
}

#[test]
fn ed25519_batch_verify_works() {
    assert_ed25519_batch_verify_works(&MockApiBech32m::new("juno"));
}
