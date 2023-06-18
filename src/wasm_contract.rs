use cosmwasm_std::CustomMsg;
use cosmwasm_vm::call_query;
use cosmwasm_vm::call_sudo;
use cosmwasm_vm::call_reply;
use cosmwasm_vm::call_migrate;
use cosmwasm_vm::call_instantiate;
use cosmwasm_vm::call_execute;


use std::collections::HashSet;
use cosmwasm_vm::internals::check_wasm;
use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cosmwasm_vm::Backend;
use cosmwasm_vm::InstanceOptions;
use cosmwasm_vm::testing::MockApi;


use cosmwasm_vm::Instance;

use crate::Contract;

use schemars::JsonSchema;


use std::fmt::{self};


use cosmwasm_std::{
    Binary, CustomQuery, Deps, DepsMut, Env, MessageInfo, Reply, Response,
};

use anyhow::{anyhow, Result as AnyResult};

// Here we create a cosmwasm-vm instance with the right definition()
pub fn mutable_cosmwasm_vm_instance<'a, T, Q: CustomQuery>(
    deps: DepsMut<Q>,
    wasm: &[u8],
) -> Instance<MockApi, &'a mut dyn cosmwasm_std::Storage, cosmwasm_std::QuerierWrapper<'a, Q>>
where 
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
    Q: CustomQuery, {
    let contract_address = MOCK_CONTRACT_ADDR;

    let backend = Backend {
        api: MockApi::default(), // TODO need to change this to validate the addresses ?
        storage: deps.storage,
        querier: deps.querier
    };
    let options = InstanceOptions {
        gas_limit:0,
        print_debug: true,
    };
    let memory_size = None;
    Instance::from_code(wasm, backend, options, memory_size).unwrap()
}

// Here we create a cosmwasm-vm instance with the right definition
pub fn cosmwasm_vm_instance<'a, T, Q: CustomQuery>(
    deps: Deps<Q>,
    wasm: &[u8],
) -> Instance<MockApi, &'a dyn cosmwasm_std::Storage, cosmwasm_std::QuerierWrapper<'a, Q>>
where 
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
    Q: CustomQuery, {
    let contract_address = MOCK_CONTRACT_ADDR;

    let backend = Backend {
        api: MockApi::default(), // TODO need to change this to validate the addresses ?
        storage: deps.storage,
        querier: deps.querier
    };
    let options = InstanceOptions {
        gas_limit:0,
        print_debug: true,
    };
    let memory_size = None;
    Instance::from_code(wasm, backend, options, memory_size).unwrap()
}



pub struct WasmContract{
    code: Vec<u8>,
    contract_addr: String,
}

impl WasmContract{
    pub fn new(code: Vec<u8>, contract_addr: String) -> Self{

        check_wasm(&code, &HashSet::default()).unwrap();
        Self{
            code,
            contract_addr
        }
    }
}



impl<T,Q> Contract<T, Q> for WasmContract
 where 
    T: CustomMsg + for<'de> serde::Deserialize<'de>,
    Q: CustomQuery,
{
    fn execute(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<T>> {

        let instance = mutable_cosmwasm_vm_instance::<T, Q>(deps, &self.code);

        call_execute(&mut instance, &env, &info, &msg).unwrap().into_result().map_err(|err| anyhow!(err))
    }

    fn instantiate(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<T>> {
        let instance = mutable_cosmwasm_vm_instance::<T, Q>(deps, &self.code);

        call_instantiate(&mut instance, &env, &info, &msg).unwrap().into_result().map_err(|err| anyhow!(err))
    }

    fn query(&self, deps: Deps<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Binary> {
        let instance = cosmwasm_vm_instance::<T, Q>(deps, &self.code);

        call_query(&mut instance, &env, &msg).unwrap().into_result().map_err(|err| anyhow!(err))
    }

    // this returns an error if the contract doesn't implement sudo
    fn sudo(&self, deps: DepsMut<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Response<T>> {
        let instance = mutable_cosmwasm_vm_instance::<T, Q>(deps, &self.code);

        call_sudo(&mut instance, &env, &msg).unwrap().into_result().map_err(|err| anyhow!(err))
    }

    // this returns an error if the contract doesn't implement reply
    fn reply(&self, deps: DepsMut<Q>, env: Env, reply_data: Reply) -> AnyResult<Response<T>> {
        let instance = mutable_cosmwasm_vm_instance::<T, Q>(deps, &self.code);

        call_reply(&mut instance, &env, &reply_data).unwrap().into_result().map_err(|err| anyhow!(err))
    }

    // this returns an error if the contract doesn't implement migrate
    fn migrate(&self, deps: DepsMut<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Response<T>> {
        let instance = mutable_cosmwasm_vm_instance::<T, Q>(deps, &self.code);

        call_migrate(&mut instance, &env, &msg).unwrap().into_result().map_err(|err| anyhow!(err))
    }
}
