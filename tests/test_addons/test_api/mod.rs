use cosmwasm_std::Api;
use hex_literal::hex;

mod test_bech32;
mod test_bech32m;

const SECP256K1_MSG_HASH: [u8; 32] =
    hex!("5ae8317d34d1e595e3fa7247db80c0af4320cce1116de187f8f7e2e099c0d8d0");
const SECP256K1_SIG: [u8; 64] = hex!("207082eb2c3dfa0b454e0906051270ba4074ac93760ba9e7110cd9471475111151eb0dbbc9920e72146fb564f99d039802bf6ef2561446eb126ef364d21ee9c4");
const SECP256K1_PUBKEY: [u8;65] = hex!("04051c1ee2190ecfb174bfe4f90763f2b4ff7517b70a2aec1876ebcfd644c4633fb03f3cfbd94b1f376e34592d9d41ccaf640bb751b00a1fadeb0c01157769eb73");
const SECP256K1_SIG_RECOVER: [u8; 64] = hex!("45c0b7f8c09a9e1f1cea0c25785594427b6bf8f9f878a8af0b1abbb48e16d0920d8becd0c220f67c51217eecfd7184ef0732481c843857e6bc7fc095c4f6b788");
const SECP256K1_PUBKEY_RECOVER: [u8;65] = hex!("044a071e8a6e10aada2b8cf39fa3b5fb3400b04e99ea8ae64ceea1a977dbeaf5d5f8c8fbd10b71ab14cd561f7df8eb6da50f8a8d81ba564342244d26d1d4211595");
const ED25519_MSG: [u8; 1] = hex!("72");
const ED25519_SIG: [u8;64] = hex!("92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00");
const ED25519_PUBKEY: [u8; 32] =
    hex!("3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c");

fn assert_secp256k1_verify_works(api: &dyn Api) {
    assert!(api
        .secp256k1_verify(&SECP256K1_MSG_HASH, &SECP256K1_SIG, &SECP256K1_PUBKEY)
        .unwrap());
}

fn assert_secp256k1_recover_pubkey_works(api: &dyn Api) {
    assert_eq!(
        api.secp256k1_recover_pubkey(&SECP256K1_MSG_HASH, &SECP256K1_SIG_RECOVER, 1)
            .unwrap(),
        SECP256K1_PUBKEY_RECOVER
    );
}

fn assert_ed25519_verify_works(api: &dyn Api) {
    assert!(api
        .ed25519_verify(&ED25519_MSG, &ED25519_SIG, &ED25519_PUBKEY)
        .unwrap());
}

fn assert_ed25519_batch_verify_works(api: &dyn Api) {
    assert!(api
        .ed25519_batch_verify(&[&ED25519_MSG], &[&ED25519_SIG], &[&ED25519_PUBKEY])
        .unwrap());
}

fn assert_debug_works(api: &dyn Api) {
    api.debug("debug works");
}
