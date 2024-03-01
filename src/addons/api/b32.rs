use bech32::{decode, encode, FromBase32, ToBase32, Variant};
use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{
    Addr, Api, CanonicalAddr, RecoverPubkeyError, StdError, StdResult, VerificationError,
};
use sha2::{Digest, Sha256};

/// Implementation of the [Api](cosmwasm_std::Api) trait that uses [Bech32] format
/// for humanizing canonical addresses.
///
/// [Bech32]: https://github.com/bitcoin/bips/blob/master/bip-0173.mediawiki
pub struct MockApiBech32 {
    api: MockApi,
    prefix: &'static str,
    variant: Variant,
}

impl MockApiBech32 {
    /// Returns `Api` implementation that uses specified prefix
    /// to generate addresses in **Bech32** format.
    ///
    /// # Example
    ///
    /// ```
    /// use cw_multi_test::addons::MockApiBech32;
    ///
    /// let api = MockApiBech32::new("juno");
    /// let addr = api.addr_make("creator");
    /// assert_eq!(addr.as_str(),
    ///            "juno1h34lmpywh4upnjdg90cjf4j70aee6z8qqfspugamjp42e4q28kqsksmtyp");
    /// ```
    pub fn new(prefix: &'static str) -> Self {
        Self::new_with_variant(prefix, Variant::Bech32)
    }

    /// Creates `Api` implementation that uses specified prefix
    /// to generate addresses in format defined by provided Bech32 variant.
    pub(crate) fn new_with_variant(prefix: &'static str, variant: Variant) -> Self {
        Self {
            api: MockApi::default(),
            prefix,
            variant,
        }
    }
}

impl Api for MockApiBech32 {
    /// Takes a human readable address in **Bech32** format and checks if it is valid.
    ///
    /// If the validation succeeds, an `Addr` containing the same string as the input is returned.
    ///
    /// # Example
    ///
    /// ```
    /// use cosmwasm_std::Api;
    /// use cw_multi_test::addons::MockApiBech32;
    ///
    /// let api = MockApiBech32::new("juno");
    /// let addr = api.addr_make("creator");
    /// assert_eq!(api.addr_validate(addr.as_str()).unwrap().as_str(),
    ///            addr.as_str());
    /// ```
    fn addr_validate(&self, input: &str) -> StdResult<Addr> {
        self.addr_humanize(&self.addr_canonicalize(input)?)
    }

    /// Takes a human readable address in **Bech32** format and returns
    /// a canonical binary representation of it.
    ///
    /// # Example
    ///
    /// ```
    /// use cosmwasm_std::Api;
    /// use cw_multi_test::addons::MockApiBech32;
    ///
    /// let api = MockApiBech32::new("juno");
    /// let addr = api.addr_make("creator");
    /// assert_eq!(api.addr_canonicalize(addr.as_str()).unwrap().to_string(),
    ///            "BC6BFD848EBD7819C9A82BF124D65E7F739D08E002601E23BB906AACD40A3D81");
    /// ```
    fn addr_canonicalize(&self, input: &str) -> StdResult<CanonicalAddr> {
        if let Ok((prefix, decoded, variant)) = decode(input) {
            if prefix == self.prefix && variant == self.variant {
                if let Ok(bytes) = Vec::<u8>::from_base32(&decoded) {
                    return Ok(bytes.into());
                }
            }
        }
        Err(StdError::generic_err("Invalid input"))
    }

    /// Takes a canonical address and returns a human readable address in **Bech32** format.
    ///
    /// This is the inverse operation of [`addr_canonicalize`].
    ///
    /// [`addr_canonicalize`]: MockApiBech32::addr_canonicalize
    ///
    /// # Example
    ///
    /// ```
    /// use cosmwasm_std::Api;
    /// use cw_multi_test::addons::MockApiBech32;
    ///
    /// let api = MockApiBech32::new("juno");
    /// let addr = api.addr_make("creator");
    /// let canonical_addr = api.addr_canonicalize(addr.as_str()).unwrap();
    /// assert_eq!(api.addr_humanize(&canonical_addr).unwrap().as_str(),
    ///            addr.as_str());
    /// ```
    fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
        if let Ok(encoded) = encode(self.prefix, canonical.as_slice().to_base32(), self.variant) {
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

impl MockApiBech32 {
    /// Returns an address in **Bech32** format, built from provided input string.
    ///
    /// # Example
    ///
    /// ```
    /// use cw_multi_test::addons::MockApiBech32;
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
        let digest = Sha256::digest(input).to_vec();
        match encode(self.prefix, digest.to_base32(), self.variant) {
            Ok(address) => Addr::unchecked(address),
            Err(reason) => panic!("Generating address failed with reason: {}", reason),
        }
    }
}
