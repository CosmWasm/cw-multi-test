#![cfg(test)]

use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

mod test_api;
mod test_app;
mod test_app_builder;
mod test_module;
mod test_prefixed_storage;
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
            to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError,
            WasmMsg,
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
