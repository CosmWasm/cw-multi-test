#![cfg(test)]

mod test_api;
mod test_app;
mod test_app_builder;
mod test_attributes;
mod test_bank;
mod test_contract_storage;
mod test_distribution;
mod test_empty_contract;
mod test_ibc;
mod test_module;
mod test_payload;
mod test_prefixed_storage;
mod test_responses;
mod test_staking;
mod test_wasm;

mod test_contracts {

    pub mod counter {
        use cosmwasm_schema::cw_serde;
        use cosmwasm_std::{
            to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError,
            WasmMsg,
        };
        use cw_multi_test::{Contract, ContractWrapper};
        use cw_storage_plus::Item;

        const COUNTER: Item<u64> = Item::new("counter");

        #[cw_serde]
        pub enum CounterQueryMsg {
            Counter {},
        }

        #[cw_serde]
        pub struct CounterResponseMsg {
            pub value: u64,
        }

        fn instantiate(
            deps: DepsMut,
            _env: Env,
            _info: MessageInfo,
            _msg: Empty,
        ) -> Result<Response, StdError> {
            COUNTER.save(deps.storage, &1)?;
            Ok(Response::default())
        }

        fn execute(
            deps: DepsMut,
            _env: Env,
            _info: MessageInfo,
            _msg: WasmMsg,
        ) -> Result<Response, StdError> {
            if let Some(mut counter) = COUNTER.may_load(deps.storage)? {
                counter += 1;
                COUNTER.save(deps.storage, &counter)?;
            }
            Ok(Response::default())
        }

        fn query(deps: Deps, _env: Env, msg: CounterQueryMsg) -> Result<Binary, StdError> {
            match msg {
                CounterQueryMsg::Counter { .. } => Ok(to_json_binary(&CounterResponseMsg {
                    value: COUNTER.may_load(deps.storage).unwrap().unwrap(),
                })?),
            }
        }

        pub fn contract() -> Box<dyn Contract<Empty>> {
            Box::new(ContractWrapper::new_with_empty(execute, instantiate, query))
        }

        #[cfg(feature = "cosmwasm_1_2")]
        pub fn contract_with_checksum() -> Box<dyn Contract<Empty>> {
            Box::new(
                ContractWrapper::new_with_empty(execute, instantiate, query).with_checksum(
                    cosmwasm_std::Checksum::generate(&[1, 2, 3, 4, 5, 6, 7, 8, 9]),
                ),
            )
        }
    }
}
