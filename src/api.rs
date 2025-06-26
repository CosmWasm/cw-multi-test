use bech32::primitives::decode::CheckedHrpstring;
use bech32::{encode, Bech32, Bech32m, Hrp};
use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{
    Addr, Api, CanonicalAddr, HashFunction, RecoverPubkeyError, StdError, StdResult,
    VerificationError,
};
use sha2::{Digest, Sha256};

pub struct MockApiBech<T> {
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

impl<T: bech32::Checksum + 'static> Api for MockApiBech<T> {
    fn addr_validate(&self, input: &str) -> StdResult<Addr> {
        self.addr_humanize(&self.addr_canonicalize(input)?)
    }

    fn addr_canonicalize(&self, input: &str) -> StdResult<CanonicalAddr> {
        if let Ok(s) = CheckedHrpstring::new::<T>(input) {
            if s.hrp().to_string() == self.prefix {
                return Ok(s.byte_iter().collect::<Vec<u8>>().into());
            }
        }
        Err(StdError::msg("Invalid input"))
    }

    fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
        let hrp = Hrp::parse(self.prefix).map_err(|e| StdError::msg(e.to_string()))?;
        if let Ok(encoded) = encode::<T>(hrp, canonical.as_slice()) {
            Ok(Addr::unchecked(encoded))
        } else {
            Err(StdError::msg("Invalid canonical address"))
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

    fn bls12_381_aggregate_g1(&self, g1s: &[u8]) -> Result<[u8; 48], VerificationError> {
        self.api.bls12_381_aggregate_g1(g1s)
    }

    fn bls12_381_aggregate_g2(&self, g2s: &[u8]) -> Result<[u8; 96], VerificationError> {
        self.api.bls12_381_aggregate_g2(g2s)
    }

    fn bls12_381_pairing_equality(
        &self,
        ps: &[u8],
        qs: &[u8],
        r: &[u8],
        s: &[u8],
    ) -> Result<bool, VerificationError> {
        self.api.bls12_381_pairing_equality(ps, qs, r, s)
    }

    fn bls12_381_hash_to_g1(
        &self,
        hash_function: HashFunction,
        msg: &[u8],
        dst: &[u8],
    ) -> Result<[u8; 48], VerificationError> {
        self.api.bls12_381_hash_to_g1(hash_function, msg, dst)
    }

    fn bls12_381_hash_to_g2(
        &self,
        hash_function: HashFunction,
        msg: &[u8],
        dst: &[u8],
    ) -> Result<[u8; 96], VerificationError> {
        self.api.bls12_381_hash_to_g2(hash_function, msg, dst)
    }

    fn secp256r1_verify(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, VerificationError> {
        self.api
            .secp256r1_verify(message_hash, signature, public_key)
    }

    fn secp256r1_recover_pubkey(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        recovery_param: u8,
    ) -> Result<Vec<u8>, RecoverPubkeyError> {
        self.api
            .secp256r1_recover_pubkey(message_hash, signature, recovery_param)
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
    /// format is not possible, especially when the prefix is too long or empty.
    pub fn addr_make(&self, input: &str) -> Addr {
        match Hrp::parse(self.prefix) {
            Ok(hrp) => Addr::unchecked(encode::<T>(hrp, Sha256::digest(input).as_slice()).unwrap()),
            Err(reason) => panic!("Generating address failed with reason: {reason}"),
        }
    }
}

/// Implementation of the `cosmwasm_std::Api` trait that uses [Bech32] format
/// for humanizing canonical addresses.
///
/// [Bech32]: https://github.com/bitcoin/bips/blob/master/bip-0173.mediawiki
pub type MockApiBech32 = MockApiBech<Bech32>;

/// Implementation of the `cosmwasm_std::Api` trait that uses [Bech32m] format
/// for humanizing canonical addresses.
///
/// [Bech32m]: https://github.com/bitcoin/bips/blob/master/bip-0350.mediawiki
pub type MockApiBech32m = MockApiBech<Bech32m>;
