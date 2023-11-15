//!

use crate::error::AnyResult;
use crate::AddressGenerator;
use cosmwasm_std::{instantiate2_address, Addr, Api, CanonicalAddr, Storage};
use sha2::digest::Update;
use sha2::{Digest, Sha256};

#[derive(Default)]
pub struct MockAddressGenerator;

impl AddressGenerator for MockAddressGenerator {
    /// prepare docs
    fn contract_address(
        &self,
        api: &dyn Api,
        _storage: &mut dyn Storage,
        code_id: u64,
        instance_id: u64,
    ) -> AnyResult<Addr> {
        let canonical_addr = instantiate_address(code_id, instance_id);
        Ok(Addr::unchecked(api.addr_humanize(&canonical_addr)?))
    }

    /// prepare docs
    fn predictable_contract_address(
        &self,
        api: &dyn Api,
        _storage: &mut dyn Storage,
        _code_id: u64,
        _instance_id: u64,
        checksum: &[u8],
        creator: &CanonicalAddr,
        salt: &[u8],
    ) -> AnyResult<Addr> {
        let canonical_addr = instantiate2_address(checksum, creator, salt)?;
        Ok(Addr::unchecked(api.addr_humanize(&canonical_addr)?))
    }
}

// Non-predictable contract address generator, see `BuildContractAddressClassic` implementation
// in wasmd: https://github.com/CosmWasm/wasmd/blob/main/x/wasm/keeper/addresses.go#L35-L42
fn instantiate_address(code_id: u64, instance_id: u64) -> CanonicalAddr {
    let mut key = Vec::<u8>::new();
    key.extend_from_slice(b"wasm\0");
    key.extend_from_slice(&code_id.to_be_bytes());
    key.extend_from_slice(&instance_id.to_be_bytes());
    let module = Sha256::digest("module".as_bytes());
    Sha256::new()
        .chain(module)
        .chain(key)
        .finalize()
        .to_vec()
        .into()
}
