use cosmwasm_std::Api;
use sha2::{Digest, Sha256};

mod test_addr;
mod test_bech32;
mod test_bech32m;
mod test_prefixed;

#[rustfmt::skip]
mod constants {
    use hex_literal::hex;

    pub const SECP256K1_MSG: [u8; 128] = hex!("5c868fedb8026979ebd26f1ba07c27eedf4ff6d10443505a96ecaf21ba8c4f0937b3cd23ffdc3dd429d4cd1905fb8dbcceeff1350020e18b58d2ba70887baa3a9b783ad30d3fbf210331cdd7df8d77defa398cdacdfc2e359c7ba4cae46bb74401deb417f8b912a1aa966aeeba9c39c7dd22479ae2b30719dca2f2206c5eb4b7");
    pub const SECP256K1_SIG: [u8; 64] = hex!("207082eb2c3dfa0b454e0906051270ba4074ac93760ba9e7110cd9471475111151eb0dbbc9920e72146fb564f99d039802bf6ef2561446eb126ef364d21ee9c4");
    pub const SECP256K1_PUBKEY: [u8; 65] = hex!("04051c1ee2190ecfb174bfe4f90763f2b4ff7517b70a2aec1876ebcfd644c4633fb03f3cfbd94b1f376e34592d9d41ccaf640bb751b00a1fadeb0c01157769eb73");
    pub const SECP256R1_MSG: [u8; 128] = hex!("5905238877c77421f73e43ee3da6f2d9e2ccad5fc942dcec0cbd25482935faaf416983fe165b1a045ee2bcd2e6dca3bdf46c4310a7461f9a37960ca672d3feb5473e253605fb1ddfd28065b53cb5858a8ad28175bf9bd386a5e471ea7a65c17cc934a9d791e91491eb3754d03799790fe2d308d16146d5c9b0d0debd97d79ce8");
    pub const SECP256R1_SIG: [u8; 64] = hex!("f3ac8061b514795b8843e3d6629527ed2afd6b1f6a555a7acabb5e6f79c8c2ac8bf77819ca05a6b2786c76262bf7371cef97b218e96f175a3ccdda2acc058903");
    pub const SECP256R1_PUBKEY: [u8; 65] = hex!("041ccbe91c075fc7f4f033bfa248db8fccd3565de94bbfb12f3c59ff46c271bf83ce4014c68811f9a21a1fdb2c0e6113e06db7ca93b7404e78dc7ccd5ca89a4ca9");
    pub const ED25519_MSG_1: [u8; 1] = hex!("72");
    pub const ED25519_MSG_2: [u8; 2] = hex!("af82");
    pub const ED25519_SIG_1: [u8; 64] = hex!("92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00");
    pub const ED25519_SIG_2: [u8; 64] = hex!("6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a");
    pub const ED25519_PUBKEY_1: [u8; 32] = hex!("3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c");
    pub const ED25519_PUBKEY_2: [u8; 32] = hex!("fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025");
}

use constants::*;

fn assert_bls12_381_aggregate_g1_works(_api: &dyn Api) {
    //TODO Add proper assertion.
}

fn assert_bls12_381_aggregate_g2_works(_api: &dyn Api) {
    //TODO Add proper assertion.
}

fn assert_bls12_381_pairing_equality_works(_api: &dyn Api) {
    //TODO Add proper assertion.
}

fn assert_bls12_381_hash_to_g1_works(_api: &dyn Api) {
    //TODO Add proper assertion.
}

fn assert_bls12_381_hash_to_g2_works(_api: &dyn Api) {
    //TODO Add proper assertion.
}

fn assert_secp256k1_verify_works(api: &dyn Api) {
    let message_hash = Sha256::digest(SECP256K1_MSG);
    assert!(api
        .secp256k1_verify(&message_hash, &SECP256K1_SIG, &SECP256K1_PUBKEY)
        .unwrap());
}

fn assert_secp256k1_recover_pubkey_works(api: &dyn Api) {
    let message_hash = Sha256::digest(SECP256K1_MSG);
    assert_eq!(
        api.secp256k1_recover_pubkey(&message_hash, &SECP256K1_SIG, 0)
            .unwrap(),
        SECP256K1_PUBKEY
    );
}

fn assert_secp256r1_verify_works(api: &dyn Api) {
    let message_hash = Sha256::digest(SECP256R1_MSG);
    assert!(api
        .secp256r1_verify(&message_hash, &SECP256R1_SIG, &SECP256R1_PUBKEY)
        .unwrap());
}

fn assert_secp256r1_recover_pubkey_works(api: &dyn Api) {
    let message_hash = Sha256::digest(SECP256R1_MSG);
    assert_eq!(
        api.secp256r1_recover_pubkey(&message_hash, &SECP256R1_SIG, 0)
            .unwrap(),
        SECP256R1_PUBKEY
    );
}

fn assert_ed25519_verify_works(api: &dyn Api) {
    assert!(api
        .ed25519_verify(&ED25519_MSG_1, &ED25519_SIG_1, &ED25519_PUBKEY_1)
        .unwrap());
}

fn assert_ed25519_batch_verify_works(api: &dyn Api) {
    assert!(api
        .ed25519_batch_verify(
            &[&ED25519_MSG_1, &ED25519_MSG_2],
            &[&ED25519_SIG_1, &ED25519_SIG_2],
            &[&ED25519_PUBKEY_1, &ED25519_PUBKEY_2]
        )
        .unwrap());
}

fn assert_debug_does_not_panic(api: &dyn Api) {
    api.debug("debug should not panic");
}
