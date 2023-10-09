use cosmwasm_std::{Addr, Api, CanonicalAddr, RecoverPubkeyError, StdResult, VerificationError};
use cw_multi_test::AppBuilder;

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

impl MyApi {
    fn my_api_extension(&self) -> &'static str {
        "my-api-extension"
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
    );
    assert_eq!(app.api().my_api_extension(), "my-api-extension");
}
