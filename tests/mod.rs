#![cfg(test)]

mod test_api;
mod test_app;
mod test_app_builder;
mod test_attributes;
mod test_bank;
mod test_contract_storage;
mod test_module;
mod test_prefixed_storage;
#[cfg(feature = "staking")]
mod test_staking;
mod test_wasm;

mod test_contracts {

    pub mod counter {
        use cosmwasm_std::{
            to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError,
            WasmMsg,
        };
        use cw_multi_test::{Contract, ContractWrapper};
        use cw_storage_plus::Item;
        use serde::{Deserialize, Serialize};

        const COUNTER: Item<u64> = Item::new("counter");

        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum CounterQueryMsg {
            Counter {},
        }

        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct CounterResponseMsg {
            pub value: u64,
        }

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
                CounterQueryMsg::Counter { .. } => Ok(to_json_binary(&CounterResponseMsg {
                    value: COUNTER.may_load(deps.storage).unwrap().unwrap(),
                })?),
            }
        }

        pub fn contract() -> Box<dyn Contract<Empty>> {
            Box::new(ContractWrapper::new_with_empty(execute, instantiate, query))
        }
    }
}
