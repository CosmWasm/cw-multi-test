use cosmwasm_std::Order;
use cosmwasm_std::Record;
use cosmwasm_vm::BackendResult;
use cosmwasm_vm::Size;
use cosmwasm_std::Binary;
use cosmwasm_std::{to_binary, from_binary};
use cosmwasm_std::Empty;
use cosmwasm_vm::testing::MockQuerier;
use cosmwasm_vm::testing::MockStorage;
use cosmwasm_vm::Querier;
use cosmwasm_vm::Storage;
use cosmwasm_vm::BackendApi;
use cosmwasm_vm::InstanceOptions;
use cosmwasm_vm::Backend;
use cosmwasm_vm::{call_execute, call_query, call_instantiate};
use cosmwasm_vm::testing::MockApi;
use cw_multi_test::wasm_emulation::output::WasmOutput;
use cw_multi_test::wasm_emulation::input::{WasmFunction, InstanceArguments, IsolatedChainData};
use cw_multi_test::wasm_emulation::output::WasmRunnerOutput;
use cw_orch::prelude::queriers::DaemonQuerier;

use cw_orch::daemon::GrpcChannel;
use cw_orch::prelude::queriers::CosmWasm;

use tokio::runtime::Runtime;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;

use cosmwasm_vm::Instance;


use anyhow::Result as AnyResult;


/// Taken from cosmwasm_vm::testing
/// This gas limit is used in integration tests and should be high enough to allow a reasonable
/// number of contract executions and queries on one instance. For this reason it is significatly
/// higher than the limit for a single execution that we have in the production setup.
const DEFAULT_GAS_LIMIT: u64 = 500_000_000_000; // ~0.5ms
const DEFAULT_MEMORY_LIMIT: Option<Size> = Some(Size::mebi(16));
const DEFAULT_PRINT_DEBUG: bool = true;

pub fn main() -> AnyResult<()>{
	// Parsing arguments (serde serialized and base64 encoded, only 1 argument)
	let args: Vec<_> = env::args().collect();
    if args.len() <= 1 {
    	panic!("The argument must be of length 1 and valid base64");
    }

    let base64_arg = &args[1];
    let InstanceArguments {chain, address, function, init_storage } = from_binary(&Binary::from_base64(base64_arg)?)?;
    let rt = Runtime::new()?;
	// We create an instance from a code_id, an address, and we run the code in it
	let channel = rt.block_on(GrpcChannel::connect(&chain.apis.grpc, &chain.chain_id))?;
	let wasm_querier = CosmWasm::new(channel);

	let code_info = rt.block_on(wasm_querier.contract_info(address.clone()))?;
	let code = rt.block_on(wasm_querier.code_data(code_info.code_id))?;

	// We create the backend here from outside information;
	let backend = Backend {
        api: MockApi::default(), // TODO need to change this to validate the addresses ?
        storage: DualStorage::new(rt, chain, address, Some(init_storage))?,
        querier: MockQuerier::<Empty>::new(&[])
    };
    let options = InstanceOptions {
        gas_limit: DEFAULT_GAS_LIMIT,
        print_debug: DEFAULT_PRINT_DEBUG,
    };
    let memory_limit = DEFAULT_MEMORY_LIMIT;

    // Then we create the instance
	let mut instance = Instance::from_code(&code, backend, options, memory_limit)?;

	// Then we call the function that we wanted to call
	let result = execute_function(&mut instance, function)?;

	// We return the code response + any storage change (or the whole local storage object), with serializing
	let mut recycled_instance = instance.recycle().unwrap();

	let wasm_result = WasmRunnerOutput{
		storage: recycled_instance.storage.get_all_storage()?,
		wasm: result,
	};

	let encoded_result = to_binary(&wasm_result)?.to_base64();
	print!("{}", encoded_result);

	Ok(())
}

fn execute_function<
	A: BackendApi + 'static, 
	B: Storage + 'static, 
	C: Querier + 'static
>
	(instance: &mut Instance<A,B,C>, function: WasmFunction) -> AnyResult<WasmOutput>{

	match function{
		WasmFunction::Execute(args) => {
			let result = call_execute(instance, &args.env, &args.info, &args.msg)?.into_result().unwrap();
			Ok(WasmOutput::Execute(result))
		},
		WasmFunction::Query(args) => {
			let result = call_query(instance, &args.env, &args.msg)?.into_result().unwrap();
			Ok(WasmOutput::Query(result))
		},
		WasmFunction::Instantiate(args) => {
			let result = call_instantiate(instance, &args.env, &args.info, &args.msg)?.into_result().unwrap();
			Ok(WasmOutput::Instantiate(result))
		},
		_ => panic!("Not implemented")
	}
}

pub struct DualStorage{
	pub local_storage: MockStorage,
	pub removed_keys: HashSet<Vec<u8>>,
	pub wasm_querier: CosmWasm,
	pub contract_addr: String,
	pub rt: Runtime,
}

impl DualStorage{
	pub fn new(rt: Runtime, chain: IsolatedChainData, contract_addr: String, init: Option<Vec<(Vec<u8>, Vec<u8>)>>) -> AnyResult<DualStorage>{
		// We create an instance from a code_id, an address, and we run the code in it
		let channel = rt.block_on(GrpcChannel::connect(&chain.apis.grpc, &chain.chain_id))?;
		let wasm_querier = CosmWasm::new(channel);

		let mut local_storage = MockStorage::default();
		for (key, value) in init.unwrap(){
			local_storage.set(&key, &value).0?;
		}

		Ok(Self{
			local_storage,
			wasm_querier,
			removed_keys: HashSet::default(),
			contract_addr,
			rt
		})
	}

	pub fn get_all_storage(&mut self) -> AnyResult<Vec<(Vec<u8>, Vec<u8>)>>{
		let iterator_id = self.local_storage.scan(None, None, Order::Ascending).0?;
		let all_records = self.local_storage.all(iterator_id);

		Ok(all_records.0?)
	}
}

impl Storage for DualStorage{
    fn get(&self, key: &[u8]) -> BackendResult<Option<Vec<u8>>>{
    	// First we try to get the value locally
    	let (mut value, gas_info) = self.local_storage.get(key);
    	// If it's not available, we query it online if it was not removed locally
    	if !self.removed_keys.contains(key) && value.as_ref().unwrap().is_none(){
    		let distant_result = self.rt.block_on(self.wasm_querier.contract_raw_state(self.contract_addr.clone(), key.to_vec()));
    		if let Ok(result) = distant_result{
    			value = Ok(Some(result.data))
    		}
    	}
    	(value, gas_info)
    }

	#[cfg(feature = "iterator")]
    fn scan(
        &mut self,
        start: Option<&[u8]>,
        end: Option<&[u8]>,
        order: Order,
    ) -> BackendResult<u32>{
    	self.local_storage.scan(start, end, order)
    }

    #[cfg(feature = "iterator")]
    fn next(&mut self, iterator_id: u32) -> BackendResult<Option<Record>>{
    	self.local_storage.next(iterator_id)
    }

    fn set(&mut self, key: &[u8], value: &[u8]) -> BackendResult<()>{
    	self.removed_keys.remove(key);
    	self.local_storage.set(key, value)
    }

    fn remove(&mut self, key: &[u8]) -> BackendResult<()>{
    	self.removed_keys.insert(key.to_vec());
    	self.local_storage.remove(key)
    }
}