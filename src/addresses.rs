//! # Implementation of address generators

use crate::error::AnyResult;
use crate::prefixed_storage::prefixed_read;
use crate::wasm::{CONTRACTS, NAMESPACE_WASM};
use cosmwasm_std::{Addr, Api, CanonicalAddr, HexBinary, Order, Storage};

/// Common address generator interface.
pub trait AddressGenerator {
    #[deprecated(
        since = "0.18.0",
        note = "use `contract_address` or `predictable_contract_address` instead; will be removed in version 1.0.0"
    )]
    fn next_address(&self, storage: &mut dyn Storage) -> Addr {
        //TODO After removing this function in version 1.0, make `CONTRACTS` and `NAMESPACE_WASM` private in `wasm.rs`.
        let count = CONTRACTS
            .range_raw(
                &prefixed_read(storage, NAMESPACE_WASM),
                None,
                None,
                Order::Ascending,
            )
            .count();
        Addr::unchecked(format!("contract{}", count))
    }

    /// Generates a _non-predictable_ contract address.
    fn contract_address(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        code_id: u64,
        instance_id: u64,
    ) -> AnyResult<Addr>;

    /// Generates a _predictable_ contract address.
    fn predictable_contract_address(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        code_id: u64,
        instance_id: u64,
        checksum: &[u8],
        creator: &CanonicalAddr,
        salt: &[u8],
    ) -> AnyResult<Addr>;
}

/// Simple contract address generator.
///
/// This generator produces fully predictable addresses,
/// no matter if [contract_address](SimpleAddressGenerator::contract_address)
/// or [predictable_contract_address](SimpleAddressGenerator::predictable_contract_address)
/// is used, but users should not make any assumptions according
/// the value of the generated addresses.
pub struct SimpleAddressGenerator();

impl AddressGenerator for SimpleAddressGenerator {
    /// Generates a contract address based on contract's instance id only.
    ///
    /// # Example
    ///
    /// ```
    /// # use cosmwasm_std::testing::{MockApi, MockStorage};
    /// # use cw_multi_test::{AddressGenerator, SimpleAddressGenerator};
    /// # let api = MockApi::default();
    /// # let mut storage = MockStorage::default();
    /// let address_generator = SimpleAddressGenerator{};
    ///
    /// let addr = address_generator.contract_address(&api, &mut storage, 100, 0).unwrap();
    /// assert_eq!(addr.to_string(),"contract0");
    ///
    /// let addr = address_generator.contract_address(&api, &mut storage, 100, 1).unwrap();
    /// assert_eq!(addr.to_string(),"contract1");
    /// ```
    fn contract_address(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _code_id: u64,
        instance_id: u64,
    ) -> AnyResult<Addr> {
        Ok(Addr::unchecked(format!("contract{instance_id}")))
    }

    /// Generates a predictable contract address based on salt only.
    ///
    /// # Example
    ///
    /// ```
    /// # use cosmwasm_std::Api;
    /// # use cosmwasm_std::testing::{MockApi, MockStorage};
    /// # use cw_multi_test::{AddressGenerator, SimpleAddressGenerator};
    /// # let api = MockApi::default();
    /// # let mut storage = MockStorage::default();
    /// # let creator = api.addr_canonicalize("creator").unwrap();
    /// let address_generator = SimpleAddressGenerator{};
    ///
    /// let addr = address_generator.predictable_contract_address(&api, &mut storage, 100, 0, &[], &creator, &[0]).unwrap();
    /// assert_eq!(addr.to_string(),"contract00");
    ///
    /// let addr = address_generator.predictable_contract_address(&api, &mut storage, 100, 1, &[], &creator, &[1]).unwrap();
    /// assert_eq!(addr.to_string(),"contract01");
    /// ```
    fn predictable_contract_address(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _code_id: u64,
        _instance_id: u64,
        _checksum: &[u8],
        _creator: &CanonicalAddr,
        salt: &[u8],
    ) -> AnyResult<Addr> {
        Ok(Addr::unchecked(format!(
            "contract{}",
            HexBinary::from(salt).to_hex()
        )))
    }
}
