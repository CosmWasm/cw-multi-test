use bech32::primitives::decode::CheckedHrpstring;
use bech32::{encode, Hrp};
use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{
    Addr, Api, CanonicalAddr, RecoverPubkeyError, StdError, StdResult, VerificationError,
};
use sha2::{Digest, Sha256};

pub mod b32;
pub mod b32m;

struct MockApiBech<T> {
    api: MockApi,
    prefix: &'static str,
    _phantom_data: std::marker::PhantomData<T>,
}

impl<T: bech32::Checksum> MockApiBech<T> {
    /// Returns `Api` implementation that uses specified prefix
    /// to generate addresses in `Bech32` or `Bech32m` format.
    pub fn new(prefix: &'static str) -> Self {
        Self {
            api: MockApi::default(),
            prefix,
            _phantom_data: std::marker::PhantomData,
        }
    }
}

impl<T: bech32::Checksum> Api for MockApiBech<T> {
    fn addr_validate(&self, input: &str) -> StdResult<Addr> {
        self.addr_humanize(&self.addr_canonicalize(input)?)
    }

    fn addr_canonicalize(&self, input: &str) -> StdResult<CanonicalAddr> {
        if let Ok(s) = CheckedHrpstring::new::<T>(input) {
            if s.hrp().to_string() == self.prefix {
                return Ok(s.byte_iter().collect::<Vec<u8>>().into());
            }
        }
        Err(StdError::generic_err("Invalid input"))
    }

    fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
        let hrp = Hrp::parse(self.prefix).map_err(|e| StdError::generic_err(e.to_string()))?;
        if let Ok(encoded) = encode::<T>(hrp, canonical.as_slice()) {
            Ok(Addr::unchecked(encoded))
        } else {
            Err(StdError::generic_err("Invalid canonical address"))
        }
    }

    fn secp256k1_verify(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        self.api
            .secp256k1_verify(message_hash, signature, public_key)
    }

    fn secp256k1_recover_pubkey(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        recovery_param: u8,
    ) -> Result<Vec<u8>, RecoverPubkeyError> {
        self.api
            .secp256k1_recover_pubkey(message_hash, signature, recovery_param)
    }

    fn ed25519_verify(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        self.api.ed25519_verify(message, signature, public_key)
    }

    fn ed25519_batch_verify(
        &self,
        messages: &[&[u8]],
        signatures: &[&[u8]],
        public_keys: &[&[u8]],
    ) -> Result<bool, VerificationError> {
        self.api
            .ed25519_batch_verify(messages, signatures, public_keys)
    }

    fn debug(&self, message: &str) {
        self.api.debug(message)
    }
}

impl<T: bech32::Checksum> MockApiBech<T> {
    /// Returns an address in `Bech32` or `Bech32m` format, built from provided input string.
    ///
    /// # Panics
    ///
    /// This function panics when generating a valid address in `Bech32` or `Bech32m`
    /// format is not possible, especially when prefix is too long or empty.
    pub fn addr_make(&self, input: &str) -> Addr {
        match Hrp::parse(self.prefix) {
            Ok(hrp) => match encode::<T>(hrp, Sha256::digest(input).as_slice()) {
                Ok(address) => Addr::unchecked(address),
                Err(reason) => panic!("Generating address failed with reason: {}", reason),
            },
            Err(reason) => panic!("Generating address failed with reason: {}", reason),
        }
    }
}
