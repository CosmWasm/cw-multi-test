


use cw_multi_test::wasm_emulation::output::WasmRunnerOutput;
use cosmwasm_std::testing::mock_info;
use cw20::Cw20ExecuteMsg;
use cw_multi_test::wasm_emulation::input::ExecuteArgs;

use cosmwasm_std::Binary;

use std::collections::HashMap;
use std::println;
use std::process::Command;

use cosmwasm_std::from_binary;
use cosmwasm_std::to_binary;
use cosmwasm_std::testing::mock_env;
use cw_multi_test::wasm_emulation::input::QueryArgs;
use cw_multi_test::wasm_emulation::input::WasmFunction;
use cw_multi_test::wasm_emulation::input::InstanceArguments;

use cw_orch::prelude::networks::PHOENIX_1;
use cw20::Cw20QueryMsg;
use anyhow::Result as AnyResult;

fn run(args: InstanceArguments) -> AnyResult<WasmRunnerOutput>{

	let serialized_args = to_binary(&args).unwrap().to_base64();

	let result = Command::new("cargo")
		.arg("run")
		.arg("-q")
		.arg("--bin")
		.arg("wasm_runner")
		.arg(serialized_args)
		.output();


	let stdout = String::from_utf8_lossy(&result.as_ref().unwrap().stdout).to_string();
	let binary_stdout = Binary::from_base64(&stdout).map(|s| from_binary(&s));
	if binary_stdout.is_err() || binary_stdout.as_ref().unwrap().is_err(){
		panic!("Err when calling contract, {:?}", result)
	}
	let decoded_result: WasmRunnerOutput = binary_stdout??;

	Ok(decoded_result)
}

type RootStorage =HashMap<String, Vec<(Vec<u8>, Vec<u8>)>>;


pub fn main(){

	// This total storage object stores everything, by contract key
	let mut storage : RootStorage = HashMap::new();
	
	let contract_addr = "terra1lxx40s29qvkrcj8fsa3yzyehy7w50umdvvnls2r830rys6lu2zns63eelv";
	let sender = "terra17c6ts8grcfrgquhj3haclg44le8s7qkx6l2yx33acguxhpf000xqhnl3je";
	let recipient = "terra1d73fvpwm8stsy5y9epfn6fedm74egn2d85xg7t";
	//storage.insert(storage_value.to_vec(), storage_key.to_vec());


	// Query :
	let query_args = InstanceArguments{
		address: "terra1lxx40s29qvkrcj8fsa3yzyehy7w50umdvvnls2r830rys6lu2zns63eelv".to_string(),
		chain: PHOENIX_1.into(),
		function: WasmFunction::Query(QueryArgs{
			env: mock_env(),
			msg: to_binary(&Cw20QueryMsg::Balance { address: recipient.to_string() }).unwrap().to_vec()
		}),
		init_storage: storage.get(contract_addr).cloned().unwrap_or(vec![])
	};

	let decoded_query_result = run(query_args).unwrap();
	println!("Balance before : {:?}", decoded_query_result);

	// We start by creating the call object

	// Execute: 
	let execute_args = InstanceArguments{
		address: contract_addr.to_string(),
		chain: PHOENIX_1.into(),
		function: WasmFunction::Execute(ExecuteArgs{
			env: mock_env(),
			info: mock_info(sender, &[]),
			msg: to_binary(&Cw20ExecuteMsg::Transfer { recipient: recipient.to_string(), amount: 1_000_000u128.into() }).unwrap().to_vec()
		}),
		init_storage: storage.get(contract_addr).cloned().unwrap_or(vec![])
	};

	let decoded_result = run(execute_args).unwrap();
	println!("Result : {:?}", decoded_result);

	storage.insert(contract_addr.to_string(), decoded_result.storage);

 
	let query_args = InstanceArguments{
		address: "terra1lxx40s29qvkrcj8fsa3yzyehy7w50umdvvnls2r830rys6lu2zns63eelv".to_string(),
		chain: PHOENIX_1.into(),
		function: WasmFunction::Query(QueryArgs{
			env: mock_env(),
			msg: to_binary(&Cw20QueryMsg::Balance { address: recipient.to_string() }).unwrap().to_vec()
		}),
		init_storage: storage.get(contract_addr).cloned().unwrap_or(vec![])
	};

	let decoded_query_result = run(query_args).unwrap();
	println!("Balance after : {:?}", decoded_query_result);


}