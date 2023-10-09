use cosmwasm_std::{Addr, Api, CanonicalAddr, RecoverPubkeyError, StdResult, VerificationError};
use cw_multi_test::{AppBuilder, TestApi};

struct MyApi {}

impl Api for MyApi {
    fn addr_validate(&self, _human: &str) -> StdResult<Addr> {
        Ok(Addr::unchecked("custom-validate"))
    }

    fn addr_canonicalize(&self, _human: &str) -> StdResult<CanonicalAddr> {
        Ok(CanonicalAddr::from(b"custom-canonicalize"))
    }

    fn addr_humanize(&self, _canonical: &CanonicalAddr) -> StdResult<Addr> {
        Ok(Addr::unchecked("custom-humanize"))
    }

    fn secp256k1_verify(
        &self,
        _message_hash: &[u8],
        _signature: &[u8],
        _public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        unimplemented!()
    }

    fn secp256k1_recover_pubkey(
        &self,
        _message_hash: &[u8],
        _signature: &[u8],
        _recovery_param: u8,
    ) -> Result<Vec<u8>, RecoverPubkeyError> {
        unimplemented!()
    }

    fn ed25519_verify(
        &self,
        _message: &[u8],
        _signature: &[u8],
        _public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        unimplemented!()
    }

    fn ed25519_batch_verify(
        &self,
        _messages: &[&[u8]],
        _signatures: &[&[u8]],
        _public_keys: &[&[u8]],
    ) -> Result<bool, VerificationError> {
        unimplemented!()
    }

    fn debug(&self, message: &str) {
        println!("{}", message)
    }
}

impl TestApi for MyApi {
    fn addr_make(&self, input: &str) -> Addr {
        Addr::unchecked(input)
    }
}

#[test]
fn building_app_with_custom_api_should_work() {
    let app = AppBuilder::default().with_api(MyApi {}).build(|_, _, _| {});
    assert_eq!(
        app.api().addr_validate("creator").unwrap(),
        Addr::unchecked("custom-validate")
    );
    assert_eq!(
        app.api().addr_canonicalize("creator").unwrap(),
        CanonicalAddr::from(b"custom-canonicalize")
    );
    assert_eq!(
        app.api().addr_humanize(&CanonicalAddr::from(&[1])).unwrap(),
        Addr::unchecked("custom-humanize")
    )
}

struct MyTestApi {}

impl Api for MyTestApi {
    fn addr_validate(&self, _human: &str) -> StdResult<Addr> {
        Ok(Addr::unchecked("custom-validate-test-api"))
    }

    fn addr_canonicalize(&self, _human: &str) -> StdResult<CanonicalAddr> {
        Ok(CanonicalAddr::from(b"custom-canonicalize-test-api"))
    }

    fn addr_humanize(&self, _canonical: &CanonicalAddr) -> StdResult<Addr> {
        Ok(Addr::unchecked("custom-humanize-test-api"))
    }

    fn secp256k1_verify(
        &self,
        _message_hash: &[u8],
        _signature: &[u8],
        _public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        unimplemented!()
    }

    fn secp256k1_recover_pubkey(
        &self,
        _message_hash: &[u8],
        _signature: &[u8],
        _recovery_param: u8,
    ) -> Result<Vec<u8>, RecoverPubkeyError> {
        unimplemented!()
    }

    fn ed25519_verify(
        &self,
        _message: &[u8],
        _signature: &[u8],
        _public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        unimplemented!()
    }

    fn ed25519_batch_verify(
        &self,
        _messages: &[&[u8]],
        _signatures: &[&[u8]],
        _public_keys: &[&[u8]],
    ) -> Result<bool, VerificationError> {
        unimplemented!()
    }

    fn debug(&self, message: &str) {
        println!("{}", message)
    }
}

impl TestApi for MyTestApi {
    fn addr_make(&self, input: &str) -> Addr {
        Addr::unchecked(input)
    }
}

#[test]
fn building_app_with_custom_test_api_should_work() {
    let app = AppBuilder::default()
        .with_api(MyTestApi {})
        .build(|_, _, _| {});
    assert_eq!(
        app.api().addr_validate("creator").unwrap(),
        Addr::unchecked("custom-validate-test-api")
    );
    assert_eq!(
        app.api().addr_canonicalize("creator").unwrap(),
        CanonicalAddr::from(b"custom-canonicalize-test-api")
    );
    assert_eq!(
        app.api().addr_humanize(&CanonicalAddr::from(&[1])).unwrap(),
        Addr::unchecked("custom-humanize-test-api")
    );
    assert_eq!(app.api().addr_make("creator"), Addr::unchecked("creator"))
}

struct MyExtendedApi {}

impl MyExtendedApi {
    fn my_extension(&self) -> &'static str {
        "my-extension"
    }
}

impl Api for MyExtendedApi {
    fn addr_validate(&self, _human: &str) -> StdResult<Addr> {
        Ok(Addr::unchecked("custom-validate-extended-api"))
    }

    fn addr_canonicalize(&self, _human: &str) -> StdResult<CanonicalAddr> {
        Ok(CanonicalAddr::from(b"custom-canonicalize-extended-api"))
    }

    fn addr_humanize(&self, _canonical: &CanonicalAddr) -> StdResult<Addr> {
        Ok(Addr::unchecked("custom-humanize-extended-api"))
    }

    fn secp256k1_verify(
        &self,
        _message_hash: &[u8],
        _signature: &[u8],
        _public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        unimplemented!()
    }

    fn secp256k1_recover_pubkey(
        &self,
        _message_hash: &[u8],
        _signature: &[u8],
        _recovery_param: u8,
    ) -> Result<Vec<u8>, RecoverPubkeyError> {
        unimplemented!()
    }

    fn ed25519_verify(
        &self,
        _message: &[u8],
        _signature: &[u8],
        _public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        unimplemented!()
    }

    fn ed25519_batch_verify(
        &self,
        _messages: &[&[u8]],
        _signatures: &[&[u8]],
        _public_keys: &[&[u8]],
    ) -> Result<bool, VerificationError> {
        unimplemented!()
    }

    fn debug(&self, message: &str) {
        println!("{}", message)
    }
}

impl TestApi for MyExtendedApi {
    fn addr_make(&self, input: &str) -> Addr {
        Addr::unchecked(input)
    }
}

#[test]
fn building_app_with_custom_extended_api_should_work() {
    let app = AppBuilder::default()
        .with_api(MyExtendedApi {})
        .build(|_, _, _| {});
    assert_eq!(
        app.api().addr_validate("creator").unwrap(),
        Addr::unchecked("custom-validate-extended-api")
    );
    assert_eq!(
        app.api().addr_canonicalize("creator").unwrap(),
        CanonicalAddr::from(b"custom-canonicalize-extended-api")
    );
    assert_eq!(
        app.api().addr_humanize(&CanonicalAddr::from(&[1])).unwrap(),
        Addr::unchecked("custom-humanize-extended-api")
    );
    assert_eq!(app.api().addr_make("creator"), Addr::unchecked("creator"));

    assert_eq!(app.api().my_extension(), "my-extension");
}
