use bech32::primitives::decode::CheckedHrpstring;
use bech32::{encode, Bech32m, Hrp};
use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{
    Addr, Api, CanonicalAddr, RecoverPubkeyError, StdError, StdResult, VerificationError,
};
use sha2::{Digest, Sha256};

/// Implementation of the `cosmwasm_std::Api` trait that uses [Bech32m] format
/// for humanizing canonical addresses.
///
/// [Bech32m]: https://github.com/bitcoin/bips/blob/master/bip-0350.mediawiki
pub struct MockApiBech32m {
    api: MockApi,
    prefix: &'static str,
}

impl MockApiBech32m {
    /// Returns `Api` implementation that uses specified prefix
    /// to generate addresses in **Bech32m** format.
    ///
    /// # Example
    ///
    /// ```
    /// use cw_multi_test::addons::MockApiBech32m;
    ///
    /// let api = MockApiBech32m::new("osmosis");
    /// let addr = api.addr_make("sender");
    /// assert_eq!(addr.as_str(),
    ///            "osmosis1pgm8hyk0pvphmlvfjc8wsvk4daluz5tgrw6pu5mfpemk74uxnx9qgv9940");
    /// ```
    pub fn new(prefix: &'static str) -> Self {
        Self {
            api: MockApi::default(),
            prefix,
        }
    }
}

impl Api for MockApiBech32m {
    /// Takes a human-readable address in **Bech32m** format and checks if it is valid.
    ///
    /// If the validation succeeds, an `Addr` containing the same string as the input is returned.
    ///
    /// # Example
    ///
    /// ```
    /// use cosmwasm_std::Api;
    /// use cw_multi_test::addons::MockApiBech32m;
    ///
    /// let api = MockApiBech32m::new("osmosis");
    /// let addr = api.addr_make("sender");
    /// assert_eq!(api.addr_validate(addr.as_str()).unwrap().as_str(),
    ///            addr.as_str());
    /// ```
    fn addr_validate(&self, input: &str) -> StdResult<Addr> {
        self.addr_humanize(&self.addr_canonicalize(input)?)
    }

    /// Takes a human-readable address in **Bech32m** format and returns
    /// a canonical binary representation of it.
    ///
    /// # Example
    ///
    /// ```
    /// use cosmwasm_std::Api;
    /// use cw_multi_test::addons::MockApiBech32;
    ///
    /// let api = MockApiBech32::new("osmosis");
    /// let addr = api.addr_make("sender");
    /// assert_eq!(api.addr_canonicalize(addr.as_str()).unwrap().to_string(),
    ///            "0A367B92CF0B037DFD89960EE832D56F7FC151681BB41E53690E776F5786998A");
    /// ```
    fn addr_canonicalize(&self, input: &str) -> StdResult<CanonicalAddr> {
        if let Ok(s) = CheckedHrpstring::new::<Bech32m>(input) {
            if s.hrp().to_string() == self.prefix {
                return Ok(s.byte_iter().collect::<Vec<u8>>().into());
            }
        }
        Err(StdError::generic_err("Invalid input"))
    }

    /// Takes a canonical address and returns a human readable address in **Bech32m** format.
    ///
    /// This is the inverse operation of [`addr_canonicalize`].
    ///
    /// [`addr_canonicalize`]: MockApiBech32m::addr_canonicalize
    ///
    /// # Example
    ///
    /// ```
    /// use cosmwasm_std::Api;
    /// use cw_multi_test::addons::MockApiBech32m;
    ///
    /// let api = MockApiBech32m::new("osmosis");
    /// let addr = api.addr_make("sender");
    /// let canonical_addr = api.addr_canonicalize(addr.as_str()).unwrap();
    /// assert_eq!(api.addr_humanize(&canonical_addr).unwrap().as_str(),
    ///            addr.as_str());
    /// ```
    fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
        if let Ok(encoded) =
            encode::<Bech32m>(Hrp::parse_unchecked(self.prefix), canonical.as_slice())
        {
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

impl MockApiBech32m {
    /// Returns an address in **Bech32m** format, built from provided input string.
    ///
    /// # Example
    ///
    /// ```
    /// use cw_multi_test::addons::MockApiBech32m;
    ///
    /// let api = MockApiBech32m::new("osmosis");
    /// let addr = api.addr_make("sender");
    /// assert_eq!(addr.as_str(),
    ///            "osmosis1pgm8hyk0pvphmlvfjc8wsvk4daluz5tgrw6pu5mfpemk74uxnx9qgv9940");
    /// ```
    ///
    /// # Panics
    ///
    /// This function panics when generating a valid address in **Bech32**
    /// format is not possible, especially when prefix is too long or empty.
    pub fn addr_make(&self, input: &str) -> Addr {
        match encode::<Bech32m>(
            Hrp::parse_unchecked(self.prefix),
            Sha256::digest(input).as_slice(),
        ) {
            Ok(address) => Addr::unchecked(address),
            Err(reason) => panic!("Generating address failed with reason: {}", reason),
        }
    }
}
