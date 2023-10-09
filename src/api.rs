use cosmwasm_std::{Addr, Api, CanonicalAddr, RecoverPubkeyError, StdResult, VerificationError};

pub trait TestApi: Api {
    fn addr_make(&self, input: &str) -> Addr;
}

#[derive(Default)]
pub struct MockApi(cosmwasm_std::testing::MockApi);

impl Api for MockApi {
    fn addr_validate(&self, input: &str) -> StdResult<Addr> {
        self.0.addr_validate(input)
    }

    fn addr_canonicalize(&self, input: &str) -> StdResult<CanonicalAddr> {
        self.0.addr_canonicalize(input)
    }

    fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
        self.0.addr_humanize(canonical)
    }

    fn secp256k1_verify(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        self.0.secp256k1_verify(message_hash, signature, public_key)
    }

    fn secp256k1_recover_pubkey(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        recovery_param: u8,
    ) -> Result<Vec<u8>, RecoverPubkeyError> {
        self.0
            .secp256k1_recover_pubkey(message_hash, signature, recovery_param)
    }

    fn ed25519_verify(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        self.0.ed25519_verify(message, signature, public_key)
    }

    fn ed25519_batch_verify(
        &self,
        messages: &[&[u8]],
        signatures: &[&[u8]],
        public_keys: &[&[u8]],
    ) -> Result<bool, VerificationError> {
        self.0
            .ed25519_batch_verify(messages, signatures, public_keys)
    }

    fn debug(&self, message: &str) {
        self.0.debug(message);
    }
}

impl TestApi for MockApi {
    #[cfg(not(feature = "cosmwasm_1_5"))]
    fn addr_make(&self, _input: &str) -> Addr {
        unimplemented!()
    }
    #[cfg(feature = "cosmwasm_1_5")]
    fn addr_make(&self, input: &str) -> Addr {
        self.0.addr_make(input)
    }
}
