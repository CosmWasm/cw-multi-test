//! Implementation of address generators.

use crate::error::AnyResult;
use cosmwasm_std::{Addr, Api, CanonicalAddr, Storage};

/// Common address generator interface.
pub trait AddressGenerator {
    #[deprecated(
        since = "0.18.0",
        note = "use classic_contract_address() or predictable_contract_address() instead, will be removed in version 1.0.0"
    )]
    fn next_address(&self, _storage: &mut dyn Storage) -> Addr {
        unimplemented!()
    }

    /// Returns a classic contract address (not predictable).
    fn classic_contract_address(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        code_id: u64,
        instance_id: u64,
    ) -> Addr;

    /// Returns a predictable contract address.
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

pub struct SimpleAddressGenerator();

impl AddressGenerator for SimpleAddressGenerator {
    fn classic_contract_address(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _code_id: u64,
        instance_id: u64,
    ) -> Addr {
        Addr::unchecked(format!("contract{}", instance_id))
    }

    fn predictable_contract_address(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _code_id: u64,
        instance_id: u64,
        _checksum: &[u8],
        _creator: &CanonicalAddr,
        _salt: &[u8],
    ) -> AnyResult<Addr> {
        Ok(Addr::unchecked(format!("contract{}", instance_id)))
    }
}
