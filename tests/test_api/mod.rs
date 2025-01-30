use base64::engine::general_purpose::STANDARD as Base64;
use base64::Engine;
use cosmwasm_std::{Api, HashFunction, BLS12_381_G1_GENERATOR};
use hex_literal::hex;
use serde::Deserialize;
use sha2::{Digest, Sha256};

mod test_addr;
mod test_bech32;
mod test_bech32m;
mod test_prefixed;

#[rustfmt::skip]
mod constants {
    use super::*;

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
    pub const ETH_BLOCK_HEADER: &[u8] = include_bytes!("./eth-block-header.json");
}

use constants::*;

fn assert_bls12_381_aggregate_g1_works(api: &dyn Api) {
    #[derive(Deserialize)]
    struct EthHeader {
        public_keys: Vec<String>,
        aggregate_pubkey: String,
    }
    let header: EthHeader = serde_json::from_slice(ETH_BLOCK_HEADER).unwrap();
    let expected = Base64.decode(header.aggregate_pubkey).unwrap();
    let pub_keys: Vec<u8> = header
        .public_keys
        .into_iter()
        .flat_map(|key| Base64.decode(key).unwrap())
        .collect();
    let actual = api.bls12_381_aggregate_g1(&pub_keys).unwrap();
    assert_eq!(expected, actual);
}

fn assert_bls12_381_aggregate_g2_works(api: &dyn Api) {
    let points: Vec<u8> = [
        hex!("b6ed936746e01f8ecf281f020953fbf1f01debd5657c4a383940b020b26507f6076334f91e2366c96e9ab279fb5158090352ea1c5b0c9274504f4f0e7053af24802e51e4568d164fe986834f41e55c8e850ce1f98458c0cfc9ab380b55285a55"),
        hex!("b23c46be3a001c63ca711f87a005c200cc550b9429d5f4eb38d74322144f1b63926da3388979e5321012fb1a0526bcd100b5ef5fe72628ce4cd5e904aeaa3279527843fae5ca9ca675f4f51ed8f83bbf7155da9ecc9663100a885d5dc6df96d9"),
        hex!("948a7cb99f76d616c2c564ce9bf4a519f1bea6b0a624a02276443c245854219fabb8d4ce061d255af5330b078d5380681751aa7053da2c98bae898edc218c75f07e24d8802a17cd1f6833b71e58f5eb5b94208b4d0bb3848cecb075ea21be115"),
    ]
        .into_iter()
        .flatten()
        .collect();
    let expected = hex!("9683b3e6701f9a4b706709577963110043af78a5b41991b998475a3d3fd62abf35ce03b33908418efc95a058494a8ae504354b9f626231f6b3f3c849dfdeaf5017c4780e2aee1850ceaf4b4d9ce70971a3d2cfcd97b7e5ecf6759f8da5f76d31");
    let actual = api.bls12_381_aggregate_g2(&points).unwrap();
    assert_eq!(expected, actual);
}

fn assert_bls12_381_pairing_equality_works(api: &dyn Api) {
    let dst = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";
    let ps = hex!("a491d1b0ecd9bb917989f0e74f0dea0422eac4a873e5e2644f368dffb9a6e20fd6e10c1b77654d067c0618f6e5a7f79ab301803f8b5ac4a1133581fc676dfedc60d891dd5fa99028805e5ea5b08d3491af75d0707adab3b70c6a6a580217bf81b53d21a4cfd562c469cc81514d4ce5a6b577d8403d32a394dc265dd190b47fa9f829fdd7963afdf972e5e77854051f6f");
    let qs: Vec<u8> = [
        hex!("0000000000000000000000000000000000000000000000000000000000000000"),
        hex!("5656565656565656565656565656565656565656565656565656565656565656"),
        hex!("abababababababababababababababababababababababababababababababab"),
    ]
    .into_iter()
    .flat_map(|msg| {
        api.bls12_381_hash_to_g2(HashFunction::Sha256, &msg, dst)
            .unwrap()
    })
    .collect();
    let s = hex!("9104e74b9dfd3ad502f25d6a5ef57db0ed7d9a0e00f3500586d8ce44231212542fcfaf87840539b398bf07626705cf1105d246ca1062c6c2e1a53029a0f790ed5e3cb1f52f8234dc5144c45fc847c0cd37a92d68e7c5ba7c648a8a339f171244");
    assert!(api
        .bls12_381_pairing_equality(&ps, &qs, &BLS12_381_G1_GENERATOR, &s)
        .unwrap());
}

fn assert_bls12_381_hash_to_g1_works(api: &dyn Api) {
    let msg = b"abc";
    let dst = b"QUUX-V01-CS02-with-BLS12381G1_XMD:SHA-256_SSWU_RO_";
    let hashed_point = api
        .bls12_381_hash_to_g1(HashFunction::Sha256, msg, dst)
        .unwrap();
    let mut serialized_expected_compressed = hex!("03567bc5ef9c690c2ab2ecdf6a96ef1c139cc0b2f284dca0a9a7943388a49a3aee664ba5379a7655d3c68900be2f6903");
    serialized_expected_compressed[0] |= 0b1000_0000;
    assert_eq!(hashed_point, serialized_expected_compressed);
}

fn assert_bls12_381_hash_to_g2_works(api: &dyn Api) {
    let msg = b"abc";
    let dst = b"QUUX-V01-CS02-with-BLS12381G2_XMD:SHA-256_SSWU_RO_";
    let hashed_point = api
        .bls12_381_hash_to_g2(HashFunction::Sha256, msg, dst)
        .unwrap();
    let mut serialized_expected_compressed = hex!("139cddbccdc5e91b9623efd38c49f81a6f83f175e80b06fc374de9eb4b41dfe4ca3a230ed250fbe3a2acf73a41177fd802c2d18e033b960562aae3cab37a27ce00d80ccd5ba4b7fe0e7a210245129dbec7780ccc7954725f4168aff2787776e6");
    serialized_expected_compressed[0] |= 0b1000_0000;
    assert_eq!(hashed_point, serialized_expected_compressed);
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
