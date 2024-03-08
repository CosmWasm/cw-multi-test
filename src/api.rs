use bech32::primitives::decode::CheckedHrpstring;
use bech32::{encode, Bech32, Bech32m, Hrp};
use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{
    Addr, Api, CanonicalAddr, RecoverPubkeyError, StdError, StdResult, VerificationError,
};
use sha2::{Digest, Sha256};

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

/// Implementation of the `cosmwasm_std::Api` trait that uses [Bech32] format
/// for humanizing canonical addresses.
///
/// [Bech32]: https://github.com/bitcoin/bips/blob/master/bip-0173.mediawiki
pub struct MockApiBech32(MockApiBech<Bech32>);

impl MockApiBech32 {
    /// Returns `Api` implementation that uses specified prefix
    /// to generate addresses in `Bech32` format.
    ///
    /// # Example
    ///
    /// ```
    /// use cw_multi_test::MockApiBech32;
    ///
    /// let api = MockApiBech32::new("juno");
    /// let addr = api.addr_make("creator");
    /// assert_eq!(addr.as_str(),
    ///            "juno1h34lmpywh4upnjdg90cjf4j70aee6z8qqfspugamjp42e4q28kqsksmtyp");
    /// ```
    pub fn new(prefix: &'static str) -> Self {
        Self(MockApiBech::new(prefix))
    }
}

impl Api for MockApiBech32 {
    /// Takes a human-readable address in `Bech32` format and checks if it is valid.
    ///
    /// If the validation succeeds, an `Addr` containing the same string as the input is returned.
    ///
    /// # Example
    ///
    /// ```
    /// use cosmwasm_std::Api;
    /// use cw_multi_test::MockApiBech32;
    ///
    /// let api = MockApiBech32::new("juno");
    /// let addr = api.addr_make("creator");
    /// assert_eq!(api.addr_validate(addr.as_str()).unwrap().as_str(),
    ///            addr.as_str());
    /// ```
    fn addr_validate(&self, input: &str) -> StdResult<Addr> {
        self.0.addr_humanize(&self.addr_canonicalize(input)?)
    }

    /// Takes a human-readable address in `Bech32` format and returns
    /// a canonical binary representation of it.
    ///
    /// # Example
    ///
    /// ```
    /// use cosmwasm_std::Api;
    /// use cw_multi_test::MockApiBech32;
    ///
    /// let api = MockApiBech32::new("juno");
    /// let addr = api.addr_make("creator");
    /// assert_eq!(api.addr_canonicalize(addr.as_str()).unwrap().to_string(),
    ///            "BC6BFD848EBD7819C9A82BF124D65E7F739D08E002601E23BB906AACD40A3D81");
    /// ```
    fn addr_canonicalize(&self, input: &str) -> StdResult<CanonicalAddr> {
        self.0.addr_canonicalize(input)
    }

    /// Takes a canonical address and returns a human-readable address in `Bech32` format.
    ///
    /// This is the inverse operation of [`addr_canonicalize`].
    ///
    /// [`addr_canonicalize`]: MockApiBech32::addr_canonicalize
    ///
    /// # Example
    ///
    /// ```
    /// use cosmwasm_std::Api;
    /// use cw_multi_test::MockApiBech32;
    ///
    /// let api = MockApiBech32::new("juno");
    /// let addr = api.addr_make("creator");
    /// let canonical_addr = api.addr_canonicalize(addr.as_str()).unwrap();
    /// assert_eq!(api.addr_humanize(&canonical_addr).unwrap().as_str(),
    ///            addr.as_str());
    /// ```
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
        self.0.debug(message)
    }
}

impl MockApiBech32 {
    /// Returns an address in `Bech32` format, built from provided input string.
    ///
    /// # Example
    ///
    /// ```
    /// use cw_multi_test::MockApiBech32;
    ///
    /// let api = MockApiBech32::new("juno");
    /// let addr = api.addr_make("creator");
    /// assert_eq!(addr.as_str(),
    ///            "juno1h34lmpywh4upnjdg90cjf4j70aee6z8qqfspugamjp42e4q28kqsksmtyp");
    /// ```
    ///
    /// # Panics
    ///
    /// This function panics when generating a valid address in **Bech32**
    /// format is not possible, especially when prefix is too long or empty.
    pub fn addr_make(&self, input: &str) -> Addr {
        self.0.addr_make(input)
    }
}

/// Implementation of the `cosmwasm_std::Api` trait that uses [Bech32m] format
/// for humanizing canonical addresses.
///
/// [Bech32m]: https://github.com/bitcoin/bips/blob/master/bip-0350.mediawiki
pub struct MockApiBech32m(MockApiBech<Bech32m>);

impl MockApiBech32m {
    /// Returns `Api` implementation that uses specified prefix
    /// to generate addresses in `Bech32m` format.
    ///
    /// # Example
    ///
    /// ```
    /// use cw_multi_test::MockApiBech32m;
    ///
    /// let api = MockApiBech32m::new("osmo");
    /// let addr = api.addr_make("sender");
    /// assert_eq!(addr.as_str(),
    ///            "osmo1pgm8hyk0pvphmlvfjc8wsvk4daluz5tgrw6pu5mfpemk74uxnx9qdlmaeg");
    /// ```
    pub fn new(prefix: &'static str) -> Self {
        Self(MockApiBech::new(prefix))
    }
}

impl Api for MockApiBech32m {
    /// Takes a human-readable address in `Bech32m` format and checks if it is valid.
    ///
    /// If the validation succeeds, an `Addr` containing the same string as the input is returned.
    ///
    /// # Example
    ///
    /// ```
    /// use cosmwasm_std::Api;
    /// use cw_multi_test::MockApiBech32m;
    ///
    /// let api = MockApiBech32m::new("osmo");
    /// let addr = api.addr_make("sender");
    /// assert_eq!(api.addr_validate(addr.as_str()).unwrap().as_str(),
    ///            addr.as_str());
    /// ```
    fn addr_validate(&self, input: &str) -> StdResult<Addr> {
        self.0.addr_humanize(&self.addr_canonicalize(input)?)
    }

    /// Takes a human-readable address in `Bech32m` format and returns
    /// a canonical binary representation of it.
    ///
    /// # Example
    ///
    /// ```
    /// use cosmwasm_std::Api;
    /// use cw_multi_test::MockApiBech32;
    ///
    /// let api = MockApiBech32::new("osmo");
    /// let addr = api.addr_make("sender");
    /// assert_eq!(api.addr_canonicalize(addr.as_str()).unwrap().to_string(),
    ///            "0A367B92CF0B037DFD89960EE832D56F7FC151681BB41E53690E776F5786998A");
    /// ```
    fn addr_canonicalize(&self, input: &str) -> StdResult<CanonicalAddr> {
        self.0.addr_canonicalize(input)
    }

    /// Takes a canonical address and returns a human-readable address in `Bech32m` format.
    ///
    /// This is the inverse operation of [`addr_canonicalize`].
    ///
    /// [`addr_canonicalize`]: MockApiBech32m::addr_canonicalize
    ///
    /// # Example
    ///
    /// ```
    /// use cosmwasm_std::Api;
    /// use cw_multi_test::MockApiBech32m;
    ///
    /// let api = MockApiBech32m::new("osmo");
    /// let addr = api.addr_make("sender");
    /// let canonical_addr = api.addr_canonicalize(addr.as_str()).unwrap();
    /// assert_eq!(api.addr_humanize(&canonical_addr).unwrap().as_str(),
    ///            addr.as_str());
    /// ```
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
        self.0.debug(message)
    }
}

impl MockApiBech32m {
    /// Returns an address in `Bech32m` format, built from provided input string.
    ///
    /// # Example
    ///
    /// ```
    /// use cw_multi_test::MockApiBech32m;
    ///
    /// let api = MockApiBech32m::new("osmo");
    /// let addr = api.addr_make("sender");
    /// assert_eq!(addr.as_str(),
    ///            "osmo1pgm8hyk0pvphmlvfjc8wsvk4daluz5tgrw6pu5mfpemk74uxnx9qdlmaeg");
    /// ```
    ///
    /// # Panics
    ///
    /// This function panics when generating a valid address in **Bech32**
    /// format is not possible, especially when prefix is too long or empty.
    pub fn addr_make(&self, input: &str) -> Addr {
        self.0.addr_make(input)
    }
}
