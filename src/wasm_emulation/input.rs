use std::collections::HashMap;

use cw_orch::daemon::ChainInfo;
use ibc_chain_registry::chain::Apis;
use ibc_chain_registry::chain::ChainData;
use cosmwasm_std::{Env, MessageInfo, Reply};
use serde::{Serialize, Deserialize};
use ibc_relayer_types::core::ics24_host::identifier::ChainId;


#[derive(Serialize, Deserialize, Debug)]
pub struct IsolatedChainData{
    pub apis: Apis,
    pub chain_id: ChainId,
}

impl From<ChainData> for IsolatedChainData{
	fn from(c: ChainData) -> Self { 
		Self{
			apis: c.apis,
			chain_id: c.chain_id
		}
	}
}
impl From<ChainInfo<'_>> for IsolatedChainData{
	fn from(c: ChainInfo) -> Self { 
		let chain_data: ChainData = c.into();
		chain_data.into()
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InstanceArguments{
	pub chain: IsolatedChainData,
	pub address: String,
	pub function: WasmFunction,
	pub init_storage: Vec<(Vec<u8>, Vec<u8>)>
}

#[derive(Serialize, Deserialize, Debug)]
pub enum WasmFunction{
	Execute(ExecuteArgs),
	Instantiate(InstantiateArgs),
	Query(QueryArgs),
	Sudo(SudoArgs),
	Reply(ReplyArgs),
	Migrate(MigrateArgs),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExecuteArgs{
	pub env: Env,
	pub info: MessageInfo,
	pub msg: Vec<u8>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InstantiateArgs{
	pub env: Env,
	pub info: MessageInfo,
	pub msg: Vec<u8>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryArgs{
	pub env: Env,
	pub msg: Vec<u8>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SudoArgs{
	pub env: Env,
	pub msg: Vec<u8>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReplyArgs{
	pub env: Env,
	pub reply: Reply
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MigrateArgs{
	pub env: Env,
	pub msg: Vec<u8>
}

