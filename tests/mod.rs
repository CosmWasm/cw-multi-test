#![cfg(test)]

use bech32::{decode, encode, FromBase32, ToBase32, Variant};
use cosmwasm_std::{
    instantiate2_address, to_binary, Addr, Api, Binary, CanonicalAddr, Deps, DepsMut, Empty, Env,
    MessageInfo, RecoverPubkeyError, Response, StdError, StdResult, Storage, VerificationError,
    WasmMsg,
};
use cw_multi_test::error::AnyResult;
use cw_multi_test::{AddressGenerator, Contract, ContractWrapper};
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

mod test_app_builder;
mod test_module;
mod test_wasm;

const COUNTER: Item<u64> = Item::new("count");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CounterQueryMsg {
    Counter {},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CounterResponseMsg {
    value: u64,
}

mod test_contracts {
    use super::*;

    pub mod counter {
        use super::*;
        use cosmwasm_std::{
            to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError, WasmMsg,
        };
        use cw_multi_test::{Contract, ContractWrapper};

        fn instantiate(
            deps: DepsMut,
            _env: Env,
            _info: MessageInfo,
            _msg: Empty,
        ) -> Result<Response, StdError> {
            COUNTER.save(deps.storage, &1).unwrap();
            Ok(Response::default())
        }

        fn execute(
            deps: DepsMut,
            _env: Env,
            _info: MessageInfo,
            _msg: WasmMsg,
        ) -> Result<Response, StdError> {
            if let Some(mut counter) = COUNTER.may_load(deps.storage).unwrap() {
                counter += 1;
                COUNTER.save(deps.storage, &counter).unwrap();
            }
            Ok(Response::default())
        }

        fn query(deps: Deps, _env: Env, msg: CounterQueryMsg) -> Result<Binary, StdError> {
            match msg {
                CounterQueryMsg::Counter { .. } => Ok(to_binary(&CounterResponseMsg {
                    value: COUNTER.may_load(deps.storage).unwrap().unwrap(),
                })?),
            }
        }

        pub fn contract() -> Box<dyn Contract<Empty>> {
            Box::new(ContractWrapper::new_with_empty(execute, instantiate, query))
        }
    }
}

mod test_api {
    use super::*;

    pub struct MockApiBech32 {
        prefix: &'static str,
    }

    impl MockApiBech32 {
        pub fn new(prefix: &'static str) -> Self {
            Self { prefix }
        }
    }

    impl Api for MockApiBech32 {
        fn addr_validate(&self, input: &str) -> StdResult<Addr> {
            let canonical = self.addr_canonicalize(input)?;
            let normalized = self.addr_humanize(&canonical)?;
            if input != normalized {
                Err(StdError::generic_err(
                    "Invalid input: address not normalized",
                ))
            } else {
                Ok(Addr::unchecked(input))
            }
        }

        fn addr_canonicalize(&self, input: &str) -> StdResult<CanonicalAddr> {
            if let Ok((prefix, decoded, Variant::Bech32)) = decode(input) {
                if prefix == self.prefix {
                    if let Ok(bytes) = Vec::<u8>::from_base32(&decoded) {
                        return Ok(bytes.into());
                    }
                }
            }
            Err(StdError::generic_err("Invalid input"))
        }

        fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
            if let Ok(encoded) = encode(
                self.prefix,
                canonical.as_slice().to_base32(),
                Variant::Bech32,
            ) {
                Ok(Addr::unchecked(encoded))
            } else {
                Err(StdError::generic_err("Invalid canonical address"))
            }
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

        fn debug(&self, _message: &str) {
            unimplemented!()
        }
    }

    impl MockApiBech32 {
        pub fn addr_make(&self, input: &str) -> Addr {
            let digest = Sha256::digest(input).to_vec();
            match encode(self.prefix, digest.to_base32(), Variant::Bech32) {
                Ok(address) => Addr::unchecked(address),
                Err(reason) => panic!("Generating address failed with reason: {reason}"),
            }
        }
    }
}

mod test_addresses {
    use super::*;

    #[derive(Default)]
    pub struct MockAddressGenerator;

    impl AddressGenerator for MockAddressGenerator {
        fn classic_contract_address(
            &self,
            api: &dyn Api,
            _storage: &mut dyn Storage,
            _code_id: u64,
            instance_id: u64,
        ) -> Addr {
            let digest = Sha256::digest(format!("contract{}", instance_id)).to_vec();
            let canonical_addr = CanonicalAddr::from(digest);
            Addr::unchecked(api.addr_humanize(&canonical_addr).unwrap())
        }

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
            Ok(Addr::unchecked(api.addr_humanize(
                &instantiate2_address(checksum, creator, salt)?,
            )?))
        }
    }
}
