use cosmwasm_std::{Response, Empty, Binary};
use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug)]
pub enum WasmOutput{
	Execute(Response<Empty>),
	Instantiate(Response<Empty>),
	Query(Binary),
	Sudo(Response<Empty>),
	Reply(Response<Empty>),
	Migrate(Response<Empty>),
}


#[derive(Serialize, Deserialize, Debug)]
pub struct WasmRunnerOutput{
	pub wasm: WasmOutput,
	pub storage: Vec<(Vec<u8>, Vec<u8>)>
}