use crate::addresses::{AddressGenerator, SimpleAddressGenerator};
use crate::app::{CosmosRouter, RouterQuerier};
use crate::checksums::{ChecksumGenerator, SimpleChecksumGenerator};
use crate::contracts::Contract;
use crate::error::{bail, AnyContext, AnyError, AnyResult, Error};
use crate::executor::AppResponse;
use crate::ibc::types::{AppIbcBasicResponse, AppIbcReceiveResponse};
use crate::prefixed_storage::{prefixed, prefixed_read, PrefixedStorage, ReadonlyPrefixedStorage};
use crate::transactions::transactional;
use cosmwasm_std::testing::mock_wasmd_attr;
#[cfg(feature = "stargate")]
use cosmwasm_std::GovMsg;
use cosmwasm_std::{
    to_json_binary, Addr, Api, Attribute, BankMsg, Binary, BlockInfo, Checksum, Coin, ContractInfo,
    ContractInfoResponse, CosmosMsg, CustomMsg, CustomQuery, Deps, DepsMut, Env, Event,
    IbcBasicResponse, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg,
    IbcChannelOpenResponse, IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg,
    IbcReceiveResponse, MessageInfo, MsgResponse, Order, Querier, QuerierWrapper, Record, Reply,
    ReplyOn, Response, StdResult, Storage, SubMsg, SubMsgResponse, SubMsgResult, TransactionInfo,
    WasmMsg, WasmQuery,
};
#[cfg(feature = "staking")]
use cosmwasm_std::{DistributionMsg, StakingMsg};
use cw_storage_plus::Map;
use prost::Message;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::fmt::Debug;

/// Contract state kept in storage, separate from the contracts themselves (contract code).
const CONTRACTS: Map<&Addr, ContractData> = Map::new("contracts");

/// Wasm module namespace.
const NAMESPACE_WASM: &[u8] = b"wasm";

/// Contract [address namespace].
///
/// [address namespace]: https://github.com/CosmWasm/wasmd/blob/96e2b91144c9a371683555f3c696f882583cc6a2/x/wasm/types/events.go#L59
const CONTRACT_ATTR: &str = "_contract_address";

/// A structure representing a privileged message.
#[derive(Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct WasmSudo {
    /// Address of a contract the privileged action will be sent to.
    pub contract_addr: Addr,
    /// Message representing privileged action to be executed by contract `sudo` entry-point.
    pub message: Binary,
}

impl WasmSudo {
    /// Creates a new privileged message for specified contract address and action to be executed.
    pub fn new<T: Serialize>(contract_addr: &Addr, msg: &T) -> StdResult<WasmSudo> {
        Ok(WasmSudo {
            contract_addr: contract_addr.clone(),
            message: to_json_binary(msg)?,
        })
    }
}

/// Contract data includes information about contract,
/// equivalent of `ContractInfo` in `wasmd` interface.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ContractData {
    /// Identifier of stored contract code
    pub code_id: u64,
    /// Address of account who initially instantiated the contract
    pub creator: Addr,
    /// Optional address of account who can execute migrations
    pub admin: Option<Addr>,
    /// Metadata passed while contract instantiation
    pub label: String,
    /// Blockchain height in the moment of instantiating the contract
    pub created: u64,
}

/// Contract code base data.
struct CodeData {
    /// Address of an account that initially stored the contract code.
    creator: Addr,
    /// Checksum of the contract's code base.
    checksum: Checksum,
    /// Identifier of the _source_ code of the contract stored in wasm keeper.
    source_id: usize,
}

/// This trait implements the interface of the Wasm module.
pub trait Wasm<ExecC, QueryC> {
    /// Handles all `WasmMsg` messages.
    fn execute(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: WasmMsg,
    ) -> AnyResult<AppResponse>;

    /// Handles all `WasmQuery` requests.
    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        request: WasmQuery,
    ) -> AnyResult<Binary>;

    /// Handles all sudo messages, this is an admin interface and can not be called via `CosmosMsg`.
    fn sudo(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        msg: WasmSudo,
    ) -> AnyResult<AppResponse>;

    /// Stores the contract's code and returns an identifier of the stored contract's code.
    fn store_code(&mut self, creator: Addr, code: Box<dyn Contract<ExecC, QueryC>>) -> u64;

    /// Stores the contract's code under specified identifier,
    /// returns the same code identifier when successful.
    fn store_code_with_id(
        &mut self,
        creator: Addr,
        code_id: u64,
        code: Box<dyn Contract<ExecC, QueryC>>,
    ) -> AnyResult<u64>;

    /// Duplicates the contract's code with specified identifier
    /// and returns an identifier of the copy of the contract's code.
    fn duplicate_code(&mut self, code_id: u64) -> AnyResult<u64>;

    /// Returns `ContractData` for the contract with specified address.
    fn contract_data(&self, storage: &dyn Storage, address: &Addr) -> AnyResult<ContractData>;

    /// Returns a raw state dump of all key-values held by a contract with specified address.
    fn dump_wasm_raw(&self, storage: &dyn Storage, address: &Addr) -> Vec<Record>;

    /// Returns the namespace of the contract storage.
    fn contract_namespace(&self, contract: &Addr) -> Vec<u8> {
        let mut name = b"contract_data/".to_vec();
        name.extend_from_slice(contract.as_bytes());
        name
    }

    /// Returns **read-only** (not mutable) contract storage.
    fn contract_storage<'a>(
        &self,
        storage: &'a dyn Storage,
        address: &Addr,
    ) -> Box<dyn Storage + 'a> {
        // We double-namespace this, once from global storage -> wasm_storage
        // then from wasm_storage -> the contracts subspace
        let namespace = self.contract_namespace(address);
        let storage = ReadonlyPrefixedStorage::multilevel(storage, &[NAMESPACE_WASM, &namespace]);
        Box::new(storage)
    }

    /// Returns **read-write** (mutable) contract storage.
    fn contract_storage_mut<'a>(
        &self,
        storage: &'a mut dyn Storage,
        address: &Addr,
    ) -> Box<dyn Storage + 'a> {
        // We double-namespace this, once from global storage -> wasm_storage
        // then from wasm_storage -> the contracts subspace
        let namespace = self.contract_namespace(address);
        let storage = PrefixedStorage::multilevel(storage, &[NAMESPACE_WASM, &namespace]);
        Box::new(storage)
    }
    /// Executes the contract ibc_channel_open endpoint
    fn ibc_channel_open(
        &self,
        _api: &dyn Api,
        _contract_addr: Addr,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _request: IbcChannelOpenMsg,
    ) -> AnyResult<IbcChannelOpenResponse> {
        panic!("No ibc channel open implemented");
    }
    /// Executes the contract ibc_channel_connect endpoint
    fn ibc_channel_connect(
        &self,
        _api: &dyn Api,
        _contract_addr: Addr,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _request: IbcChannelConnectMsg,
    ) -> AnyResult<AppIbcBasicResponse> {
        panic!("No ibc channel connect implemented");
    }

    /// Executes the contract ibc_channel_close endpoint
    fn ibc_channel_close(
        &self,
        _api: &dyn Api,
        _contract_addr: Addr,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _request: IbcChannelCloseMsg,
    ) -> AnyResult<AppIbcBasicResponse> {
        panic!("No ibc channel close implemented");
    }

    /// Executes the contract ibc_packet_receive endpoint
    fn ibc_packet_receive(
        &self,
        _api: &dyn Api,
        _contract_addr: Addr,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _request: IbcPacketReceiveMsg,
    ) -> AnyResult<AppIbcReceiveResponse> {
        panic!("No ibc packet receive implemented");
    }

    /// Executes the contract ibc_packet_acknowledge endpoint
    fn ibc_packet_acknowledge(
        &self,
        _api: &dyn Api,
        _contract_addr: Addr,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _request: IbcPacketAckMsg,
    ) -> AnyResult<AppIbcBasicResponse> {
        panic!("No ibc packet acknowledgement implemented");
    }

    /// Executes the contract ibc_packet_timeout endpoint
    fn ibc_packet_timeout(
        &self,
        _api: &dyn Api,
        _contract_addr: Addr,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _request: IbcPacketTimeoutMsg,
    ) -> AnyResult<AppIbcBasicResponse> {
        panic!("No ibc packet timeout implemented");
    }
}

/// A structure representing a default wasm keeper.
pub struct WasmKeeper<ExecC, QueryC> {
    /// Contract codes that stand for wasm code in real-life blockchain.
    code_base: Vec<Box<dyn Contract<ExecC, QueryC>>>,
    /// Code data with code base identifier and additional attributes.
    code_data: BTreeMap<u64, CodeData>,
    /// Contract's address generator.
    address_generator: Box<dyn AddressGenerator>,
    /// Contract's code checksum generator.
    checksum_generator: Box<dyn ChecksumGenerator>,
    /// Just markers to make type elision fork when using it as `Wasm` trait
    _p: std::marker::PhantomData<QueryC>,
}

impl<ExecC, QueryC> Default for WasmKeeper<ExecC, QueryC> {
    /// Returns the default value for [WasmKeeper].
    fn default() -> Self {
        Self {
            code_base: Vec::default(),
            code_data: BTreeMap::default(),
            address_generator: Box::new(SimpleAddressGenerator),
            checksum_generator: Box::new(SimpleChecksumGenerator),
            _p: std::marker::PhantomData,
        }
    }
}

impl<ExecC, QueryC> Wasm<ExecC, QueryC> for WasmKeeper<ExecC, QueryC>
where
    ExecC: CustomMsg + DeserializeOwned + 'static,
    QueryC: CustomQuery + DeserializeOwned + 'static,
{
    fn execute(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: WasmMsg,
    ) -> AnyResult<AppResponse> {
        self.execute_wasm(api, storage, router, block, sender.clone(), msg.clone())
            .context(format!(
                "Error executing WasmMsg:\n  sender: {}\n  {:?}",
                sender, msg
            ))
    }

    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        request: WasmQuery,
    ) -> AnyResult<Binary> {
        match request {
            WasmQuery::Smart { contract_addr, msg } => {
                let addr = api.addr_validate(&contract_addr)?;
                self.query_smart(addr, api, storage, querier, block, msg.into())
            }
            WasmQuery::Raw { contract_addr, key } => {
                let addr = api.addr_validate(&contract_addr)?;
                Ok(self.query_raw(addr, storage, &key))
            }
            WasmQuery::ContractInfo { contract_addr } => {
                let addr = api.addr_validate(&contract_addr)?;
                let contract = self.contract_data(storage, &addr)?;
                let res = ContractInfoResponse::new(
                    contract.code_id,
                    contract.creator,
                    contract.admin,
                    false,
                    None,
                );
                to_json_binary(&res).map_err(Into::into)
            }
            #[cfg(feature = "cosmwasm_1_2")]
            WasmQuery::CodeInfo { code_id } => {
                let code_data = self.code_data(code_id)?;
                let res = cosmwasm_std::CodeInfoResponse::new(
                    code_id,
                    code_data.creator.clone(),
                    code_data.checksum,
                );
                to_json_binary(&res).map_err(Into::into)
            }
            _ => unimplemented!("{}", Error::unsupported_wasm_query(request)),
        }
    }

    fn sudo(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        msg: WasmSudo,
    ) -> AnyResult<AppResponse> {
        let custom_event = Event::new("sudo").add_attribute(CONTRACT_ATTR, &msg.contract_addr);
        let res = self.call_sudo(
            msg.contract_addr.clone(),
            api,
            storage,
            router,
            block,
            msg.message.to_vec(),
        )?;
        let (res, msgs) = self.build_app_response(&msg.contract_addr, custom_event, res);
        self.process_response(api, router, storage, block, msg.contract_addr, res, msgs)
    }

    /// Stores the contract's code in the in-memory lookup table.
    /// Returns an identifier of the stored contract code.
    fn store_code(&mut self, creator: Addr, code: Box<dyn Contract<ExecC, QueryC>>) -> u64 {
        let code_id = self
            .next_code_id()
            .unwrap_or_else(|| panic!("{}", Error::NoMoreCodeIdAvailable));
        self.save_code(code_id, creator, code)
    }

    /// Stores the contract's code in the in-memory lookup table.
    /// Returns an identifier of the stored contract code.
    fn store_code_with_id(
        &mut self,
        creator: Addr,
        code_id: u64,
        code: Box<dyn Contract<ExecC, QueryC>>,
    ) -> AnyResult<u64> {
        // validate provided contract code identifier
        if self.code_data.contains_key(&code_id) {
            bail!(Error::duplicated_code_id(code_id));
        } else if code_id == 0 {
            bail!(Error::invalid_code_id());
        }
        Ok(self.save_code(code_id, creator, code))
    }

    /// Duplicates the contract's code with specified identifier.
    /// Returns an identifier of the copy of the contract's code.
    fn duplicate_code(&mut self, code_id: u64) -> AnyResult<u64> {
        let code_data = self.code_data(code_id)?;
        let new_code_id = self
            .next_code_id()
            .ok_or_else(Error::no_more_code_id_available)?;
        self.code_data.insert(
            new_code_id,
            CodeData {
                creator: code_data.creator.clone(),
                checksum: code_data.checksum,
                source_id: code_data.source_id,
            },
        );
        Ok(new_code_id)
    }

    /// Returns `ContractData` for the contract with specified address.
    fn contract_data(&self, storage: &dyn Storage, address: &Addr) -> AnyResult<ContractData> {
        CONTRACTS
            .load(&prefixed_read(storage, NAMESPACE_WASM), address)
            .map_err(Into::into)
    }

    /// Returns a raw state dump of all key-values held by a contract with specified address.
    fn dump_wasm_raw(&self, storage: &dyn Storage, address: &Addr) -> Vec<Record> {
        let storage = self.contract_storage(storage, address);
        storage.range(None, None, Order::Ascending).collect()
    }

    // The following ibc endpoints can only be used by the ibc module.
    // For channels
    fn ibc_channel_open(
        &self,
        api: &dyn Api,
        contract: Addr,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        request: IbcChannelOpenMsg,
    ) -> AnyResult<IbcChannelOpenResponse> {
        // For channel open, we simply return the result directly to the ibc module
        let contract_response = self.with_storage(
            api,
            storage,
            router,
            block,
            contract,
            |contract, deps, env| contract.ibc_channel_open(deps, env, request),
        )?;

        Ok(contract_response)
    }

    fn ibc_channel_connect(
        &self,
        api: &dyn Api,
        contract_addr: Addr,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        request: IbcChannelConnectMsg,
    ) -> AnyResult<AppIbcBasicResponse> {
        let res = Self::verify_ibc_response(self.with_storage(
            api,
            storage,
            router,
            block,
            contract_addr.clone(),
            |contract, deps, env| contract.ibc_channel_connect(deps, env, request),
        )?)?;

        self.process_ibc_response(api, contract_addr, storage, router, block, res)
    }

    fn ibc_channel_close(
        &self,
        api: &dyn Api,
        contract_addr: Addr,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        request: IbcChannelCloseMsg,
    ) -> AnyResult<AppIbcBasicResponse> {
        let res = Self::verify_ibc_response(self.with_storage(
            api,
            storage,
            router,
            block,
            contract_addr.clone(),
            |contract, deps, env| contract.ibc_channel_close(deps, env, request),
        )?)?;

        self.process_ibc_response(api, contract_addr, storage, router, block, res)
    }

    fn ibc_packet_receive(
        &self,
        api: &dyn Api,
        contract_addr: Addr,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        request: IbcPacketReceiveMsg,
    ) -> AnyResult<AppIbcReceiveResponse> {
        let res = Self::verify_packet_response(self.with_storage(
            api,
            storage,
            router,
            block,
            contract_addr.clone(),
            |contract, deps, env| contract.ibc_packet_receive(deps, env, request),
        )?)?;

        self.process_ibc_receive_response(api, contract_addr, storage, router, block, res)
    }

    fn ibc_packet_acknowledge(
        &self,
        api: &dyn Api,
        contract_addr: Addr,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        request: IbcPacketAckMsg,
    ) -> AnyResult<AppIbcBasicResponse> {
        let res = Self::verify_ibc_response(self.with_storage(
            api,
            storage,
            router,
            block,
            contract_addr.clone(),
            |contract, deps, env| contract.ibc_packet_acknowledge(deps, env, request),
        )?)?;

        self.process_ibc_response(api, contract_addr, storage, router, block, res)
    }

    fn ibc_packet_timeout(
        &self,
        api: &dyn Api,
        contract_addr: Addr,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        request: IbcPacketTimeoutMsg,
    ) -> AnyResult<AppIbcBasicResponse> {
        let res = Self::verify_ibc_response(self.with_storage(
            api,
            storage,
            router,
            block,
            contract_addr.clone(),
            |contract, deps, env| contract.ibc_packet_timeout(deps, env, request),
        )?)?;

        self.process_ibc_response(api, contract_addr, storage, router, block, res)
    }
}

impl<ExecC, QueryC> WasmKeeper<ExecC, QueryC>
where
    ExecC: CustomMsg + DeserializeOwned + 'static,
    QueryC: CustomQuery + DeserializeOwned + 'static,
{
    /// Creates a wasm keeper with default settings.
    ///
    /// # Example
    ///
    /// ```
    /// use cw_multi_test::{no_init, AppBuilder, WasmKeeper};
    ///
    /// // create wasm keeper
    /// let wasm_keeper = WasmKeeper::new();
    ///
    /// // create and use the application with newly created wasm keeper
    /// let mut app = AppBuilder::default().with_wasm(wasm_keeper).build(no_init);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Populates an existing [WasmKeeper] with custom contract address generator.
    ///
    /// # Example
    ///
    /// ```
    /// use cosmwasm_std::{Addr, Api, Storage};
    /// use cw_multi_test::{no_init, AddressGenerator, AppBuilder, WasmKeeper};
    /// use cw_multi_test::error::AnyResult;
    /// # use cosmwasm_std::testing::MockApi;
    ///
    /// struct CustomAddressGenerator;
    ///
    /// impl AddressGenerator for CustomAddressGenerator {
    ///     fn contract_address(
    ///         &self,
    ///         api: &dyn Api,
    ///         storage: &mut dyn Storage,
    ///         code_id: u64,
    ///         instance_id: u64,
    ///     ) -> AnyResult<Addr> {
    ///         // here implement your address generation logic
    /// #       Ok(MockApi::default().addr_make("test_address"))
    ///     }
    /// }
    ///
    /// // populate wasm with your custom address generator
    /// let wasm_keeper = WasmKeeper::new().with_address_generator(CustomAddressGenerator);
    ///
    /// // create and use the application with customized wasm keeper
    /// let mut app = AppBuilder::default().with_wasm(wasm_keeper).build(no_init);
    /// ```
    pub fn with_address_generator(
        mut self,
        address_generator: impl AddressGenerator + 'static,
    ) -> Self {
        self.address_generator = Box::new(address_generator);
        self
    }

    /// Populates an existing [WasmKeeper] with custom checksum generator for the contract code.
    ///
    /// # Example
    ///
    /// ```
    /// use cosmwasm_std::{Addr, Checksum};
    /// use cw_multi_test::{no_init, AppBuilder, ChecksumGenerator, WasmKeeper};
    ///
    /// struct MyChecksumGenerator;
    ///
    /// impl ChecksumGenerator for MyChecksumGenerator {
    ///     fn checksum(&self, creator: &Addr, code_id: u64) -> Checksum {
    ///         // here implement your custom checksum generator
    /// #       Checksum::from_hex("custom_checksum").unwrap()
    ///     }
    /// }
    ///
    /// // populate wasm keeper with your custom checksum generator
    /// let wasm_keeper = WasmKeeper::new().with_checksum_generator(MyChecksumGenerator);
    ///
    /// // create and use the application with customized wasm keeper
    /// let mut app = AppBuilder::default().with_wasm(wasm_keeper).build(no_init);
    /// ```
    pub fn with_checksum_generator(
        mut self,
        checksum_generator: impl ChecksumGenerator + 'static,
    ) -> Self {
        self.checksum_generator = Box::new(checksum_generator);
        self
    }

    /// Returns a handler to code of the contract with specified code id.
    pub fn contract_code(&self, code_id: u64) -> AnyResult<&dyn Contract<ExecC, QueryC>> {
        let code_data = self.code_data(code_id)?;
        Ok(self.code_base[code_data.source_id].borrow())
    }

    /// Returns code data of the contract with specified code id.
    fn code_data(&self, code_id: u64) -> AnyResult<&CodeData> {
        if code_id < 1 {
            bail!(Error::invalid_code_id());
        }
        Ok(self
            .code_data
            .get(&code_id)
            .ok_or_else(|| Error::unregistered_code_id(code_id))?)
    }

    /// Validates all attributes.
    ///
    /// In `wasmd`, before version v0.45.0 empty attribute values were not allowed.
    /// Since `wasmd` v0.45.0 empty attribute values are allowed,
    /// so the value is not validated anymore.
    fn verify_attributes(attributes: &[Attribute]) -> AnyResult<()> {
        for attr in attributes {
            let key = attr.key.trim();
            let val = attr.value.trim();
            if key.is_empty() {
                bail!(Error::empty_attribute_key(val));
            }
            if key.starts_with('_') {
                bail!(Error::reserved_attribute_key(key));
            }
        }
        Ok(())
    }

    fn verify_ibc_response<T>(response: IbcBasicResponse<T>) -> AnyResult<IbcBasicResponse<T>>
    where
        T: Clone + Debug + PartialEq + JsonSchema,
    {
        Self::verify_attributes(&response.attributes)?;

        for event in &response.events {
            Self::verify_attributes(&event.attributes)?;
            let ty = event.ty.trim();
            if ty.len() < 2 {
                bail!(Error::event_type_too_short(ty));
            }
        }

        Ok(response)
    }

    fn verify_packet_response<T>(
        response: IbcReceiveResponse<T>,
    ) -> AnyResult<IbcReceiveResponse<T>>
    where
        T: Clone + Debug + PartialEq + JsonSchema,
    {
        Self::verify_attributes(&response.attributes)?;

        for event in &response.events {
            Self::verify_attributes(&event.attributes)?;
            let ty = event.ty.trim();
            if ty.len() < 2 {
                bail!(Error::event_type_too_short(ty));
            }
        }

        Ok(response)
    }

    fn verify_response<T>(response: Response<T>) -> AnyResult<Response<T>>
    where
        T: CustomMsg,
    {
        Self::verify_attributes(&response.attributes)?;

        for event in &response.events {
            Self::verify_attributes(&event.attributes)?;
            let ty = event.ty.trim();
            if ty.len() < 2 {
                bail!(Error::event_type_too_short(ty));
            }
        }

        Ok(response)
    }

    fn save_code(
        &mut self,
        code_id: u64,
        creator: Addr,
        code: Box<dyn Contract<ExecC, QueryC>>,
    ) -> u64 {
        // prepare the next identifier for the contract's code
        let source_id = self.code_base.len();
        // prepare the contract's Wasm blob checksum
        let checksum = code
            .checksum()
            .unwrap_or(self.checksum_generator.checksum(&creator, code_id));
        // store the 'source' code of the contract
        self.code_base.push(code);
        // store the additional code attributes like creator address and checksum
        self.code_data.insert(
            code_id,
            CodeData {
                creator,
                checksum,
                source_id,
            },
        );
        code_id
    }

    /// Returns the next contract's code identifier.
    fn next_code_id(&self) -> Option<u64> {
        self.code_data.keys().last().unwrap_or(&0u64).checked_add(1)
    }

    /// Executes the contract's `query` entry-point.
    pub fn query_smart(
        &self,
        address: Addr,
        api: &dyn Api,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Binary> {
        self.with_storage_readonly(
            api,
            storage,
            querier,
            block,
            address,
            |handler, deps, env| handler.query(deps, env, msg),
        )
    }

    /// Returns the value stored under specified key in contracts storage.
    pub fn query_raw(&self, address: Addr, storage: &dyn Storage, key: &[u8]) -> Binary {
        let storage = self.contract_storage(storage, &address);
        let data = storage.get(key).unwrap_or_default();
        data.into()
    }

    fn send<T>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: T,
        recipient: String,
        amount: &[Coin],
    ) -> AnyResult<AppResponse>
    where
        T: Into<Addr>,
    {
        if !amount.is_empty() {
            let msg: CosmosMsg<ExecC> = BankMsg::Send {
                to_address: recipient,
                amount: amount.to_vec(),
            }
            .into();
            let res = router.execute(api, storage, block, sender.into(), msg)?;
            Ok(res)
        } else {
            Ok(AppResponse::default())
        }
    }

    /// unified logic for UpdateAdmin and ClearAdmin messages
    fn update_admin(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        sender: Addr,
        contract_addr: &str,
        new_admin: Option<String>,
    ) -> AnyResult<AppResponse> {
        let contract_addr = api.addr_validate(contract_addr)?;
        let admin = new_admin.map(|a| api.addr_validate(&a)).transpose()?;

        // check admin status
        let mut contract_data = self.contract_data(storage, &contract_addr)?;
        if contract_data.admin != Some(sender) {
            bail!(
                "Only admin can update the contract admin: {:?}",
                contract_data.admin
            );
        }
        // update admin field
        contract_data.admin = admin;
        self.save_contract(storage, &contract_addr, &contract_data)?;

        // No custom events or data here.
        Ok(AppResponse::default())
    }

    // this returns the contract address as well, so we can properly resend the data
    fn execute_wasm(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: WasmMsg,
    ) -> AnyResult<AppResponse> {
        match msg {
            WasmMsg::Execute {
                contract_addr,
                msg,
                funds,
            } => {
                let contract_addr = api.addr_validate(&contract_addr)?;
                // first move the cash
                self.send(
                    api,
                    storage,
                    router,
                    block,
                    sender.clone(),
                    contract_addr.clone().into(),
                    &funds,
                )?;

                // then call the contract
                let info = MessageInfo { sender, funds };
                let response = self.call_execute(
                    api,
                    storage,
                    contract_addr.clone(),
                    router,
                    block,
                    info,
                    msg.to_vec(),
                )?;

                let custom_event =
                    Event::new("execute").add_attribute(CONTRACT_ATTR, &contract_addr);

                let (sub_response, sub_messages) =
                    self.build_app_response(&contract_addr, custom_event, response);

                let mut app_response = self.process_response(
                    api,
                    router,
                    storage,
                    block,
                    contract_addr,
                    sub_response,
                    sub_messages,
                )?;
                app_response.data = encode_response_data(app_response.data);
                Ok(app_response)
            }
            WasmMsg::Instantiate {
                admin,
                code_id,
                msg,
                funds,
                label,
            } => self.process_wasm_msg_instantiate(
                api, storage, router, block, sender, admin, code_id, msg, funds, label, None,
            ),
            #[cfg(feature = "cosmwasm_1_2")]
            WasmMsg::Instantiate2 {
                admin,
                code_id,
                msg,
                funds,
                label,
                salt,
            } => self.process_wasm_msg_instantiate(
                api,
                storage,
                router,
                block,
                sender,
                admin,
                code_id,
                msg,
                funds,
                label,
                Some(salt),
            ),
            WasmMsg::Migrate {
                contract_addr,
                new_code_id,
                msg,
            } => {
                let contract_addr = api.addr_validate(&contract_addr)?;

                // check admin status and update the stored code_id
                if new_code_id as usize > self.code_data.len() {
                    bail!("Cannot migrate contract to unregistered code id");
                }
                let mut data = self.contract_data(storage, &contract_addr)?;
                if data.admin != Some(sender) {
                    bail!("Only admin can migrate contract: {:?}", data.admin);
                }
                data.code_id = new_code_id;
                self.save_contract(storage, &contract_addr, &data)?;

                // then call migrate
                let res = self.call_migrate(
                    contract_addr.clone(),
                    api,
                    storage,
                    router,
                    block,
                    msg.to_vec(),
                )?;

                let custom_event = Event::new("migrate")
                    .add_attribute(CONTRACT_ATTR, &contract_addr)
                    .add_attribute("code_id", new_code_id.to_string());
                let (res, msgs) = self.build_app_response(&contract_addr, custom_event, res);
                let mut res =
                    self.process_response(api, router, storage, block, contract_addr, res, msgs)?;
                res.data = encode_response_data(res.data);
                Ok(res)
            }
            WasmMsg::UpdateAdmin {
                contract_addr,
                admin,
            } => self.update_admin(api, storage, sender, &contract_addr, Some(admin)),
            WasmMsg::ClearAdmin { contract_addr } => {
                self.update_admin(api, storage, sender, &contract_addr, None)
            }
            _ => unimplemented!("{}", Error::unsupported_wasm_message(msg)),
        }
    }

    /// Processes WasmMsg::Instantiate and WasmMsg::Instantiate2 messages.
    fn process_wasm_msg_instantiate(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        admin: Option<String>,
        code_id: u64,
        msg: Binary,
        funds: Vec<Coin>,
        label: String,
        salt: Option<Binary>,
    ) -> AnyResult<AppResponse> {
        if label.is_empty() {
            bail!("Label is required on all contracts");
        }

        let contract_addr = self.register_contract(
            api,
            storage,
            code_id,
            sender.clone(),
            admin.map(Addr::unchecked),
            label,
            block.height,
            salt,
        )?;

        // move the cash
        self.send(
            api,
            storage,
            router,
            block,
            sender.clone(),
            contract_addr.clone().into(),
            &funds,
        )?;

        // then call the contract
        let info = MessageInfo { sender, funds };
        let res = self.call_instantiate(
            contract_addr.clone(),
            api,
            storage,
            router,
            block,
            info,
            msg.to_vec(),
        )?;

        let custom_event = Event::new("instantiate")
            .add_attribute(CONTRACT_ATTR, &contract_addr)
            .add_attribute("code_id", code_id.to_string());

        let (res, msgs) = self.build_app_response(&contract_addr, custom_event, res);
        let mut res = self.process_response(
            api,
            router,
            storage,
            block,
            contract_addr.clone(),
            res,
            msgs,
        )?;
        res.data = Some(instantiate_response(res.data, &contract_addr));
        Ok(res)
    }

    /// This will execute the given messages, making all changes to the local cache.
    /// This *will* write some data to the cache if the message fails half-way through.
    /// All sequential calls to RouterCache will be one atomic unit (all commit or all fail).
    ///
    /// For normal use cases, you can use Router::execute() or Router::execute_multi().
    /// This is designed to be handled internally as part of larger process flows.
    ///
    /// The `data` on `AppResponse` is data returned from `reply` call, not from execution of
    /// sub-message itself. In case if `reply` is not called, no `data` is set.
    fn execute_submsg(
        &self,
        api: &dyn Api,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        contract: Addr,
        msg: SubMsg<ExecC>,
    ) -> AnyResult<AppResponse> {
        let SubMsg {
            msg,
            id,
            reply_on,
            payload,
            ..
        } = msg;
        // Prepare the message type URL, will be needed when calling `reply` entrypoint.
        let type_url = Self::response_type_url(&msg);

        // Execute the submessage in cache
        let sub_message_result = transactional(storage, |write_cache, _| {
            router.execute(api, write_cache, block, contract.clone(), msg)
        });

        // call reply if meaningful
        if let Ok(mut r) = sub_message_result {
            if matches!(reply_on, ReplyOn::Always | ReplyOn::Success) {
                let reply = Reply {
                    id,
                    payload,
                    gas_used: 0,
                    result: SubMsgResult::Ok(
                        #[allow(deprecated)]
                        SubMsgResponse {
                            events: r.events.clone(),
                            data: r.data.clone(),
                            msg_responses: vec![MsgResponse {
                                type_url,
                                value: r.data.unwrap_or_default(),
                            }],
                        },
                    ),
                };
                // do reply and combine it with the original response
                let reply_res = self.reply(api, router, storage, block, contract, reply)?;
                // override data
                r.data = reply_res.data;
                // append the events
                r.events.extend_from_slice(&reply_res.events);
            } else {
                // reply is not called, no data should be returned
                r.data = None;
            }
            Ok(r)
        } else if let Err(e) = sub_message_result {
            if matches!(reply_on, ReplyOn::Always | ReplyOn::Error) {
                let reply = Reply {
                    id,
                    payload,
                    gas_used: 0,
                    result: SubMsgResult::Err(format!("{:?}", e)),
                };
                self.reply(api, router, storage, block, contract, reply)
            } else {
                Err(e)
            }
        } else {
            sub_message_result
        }
    }

    fn reply(
        &self,
        api: &dyn Api,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        contract: Addr,
        reply: Reply,
    ) -> AnyResult<AppResponse> {
        let ok_attr = if reply.result.is_ok() {
            "handle_success"
        } else {
            "handle_failure"
        };
        let custom_event = Event::new("reply")
            .add_attribute(CONTRACT_ATTR, &contract)
            .add_attribute("mode", ok_attr);

        let res = self.call_reply(contract.clone(), api, storage, router, block, reply)?;
        let (res, msgs) = self.build_app_response(&contract, custom_event, res);
        self.process_response(api, router, storage, block, contract, res, msgs)
    }

    /// Captures all the events, data and sub messages from the contract call.
    ///
    /// This function does not handle the messages.
    fn build_app_response(
        &self,
        contract: &Addr,
        custom_event: Event, // entry-point specific custom event added by x/wasm
        response: Response<ExecC>,
    ) -> (AppResponse, Vec<SubMsg<ExecC>>) {
        let Response {
            messages,
            attributes,
            events,
            data,
            ..
        } = response;

        // always add custom event
        let mut app_events = Vec::with_capacity(2 + events.len());
        app_events.push(custom_event);

        // we only emit the `wasm` event if some attributes are specified
        if !attributes.is_empty() {
            // turn attributes into event and place it first
            let wasm_event = Event::new("wasm")
                .add_attribute(CONTRACT_ATTR, contract)
                .add_attributes(attributes);
            app_events.push(wasm_event);
        }

        // These need to get `wasm-` prefix to match the wasmd semantics (custom wasm messages cannot
        // fake system level event types, like transfer from the bank module)
        let wasm_events = events.into_iter().map(|mut ev| {
            ev.ty = format!("wasm-{}", ev.ty);
            ev.attributes
                .insert(0, mock_wasmd_attr(CONTRACT_ATTR, contract));
            ev
        });
        app_events.extend(wasm_events);

        let app_response = AppResponse {
            events: app_events,
            data,
        };
        (app_response, messages)
    }

    fn process_response(
        &self,
        api: &dyn Api,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        contract: Addr,
        response: AppResponse,
        sub_messages: Vec<SubMsg<ExecC>>,
    ) -> AnyResult<AppResponse> {
        // Unpack the provided response.
        let AppResponse {
            mut events, data, ..
        } = response;
        // Recurse in all submessages.
        let data = sub_messages
            .into_iter()
            .try_fold(data, |data, sub_message| {
                // Execute the submessage.
                let sub_response = self.execute_submsg(
                    api,
                    router,
                    storage,
                    block,
                    contract.clone(),
                    sub_message,
                )?;
                // COLLECT and append all events from the processed submessage.
                events.extend_from_slice(&sub_response.events);
                // REPLACE the data with value from the processes submessage (if not empty).
                Ok::<_, AnyError>(sub_response.data.or(data))
            })?;
        // Return the response with updated data, events and message responses taken from
        // all processed sub messages. Note that events and message responses are collected,
        // but the data is replaced with the data from the last processes submessage.
        Ok(AppResponse { events, data })
    }

    fn process_ibc_response(
        &self,
        api: &dyn Api,
        contract: Addr,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        res: IbcBasicResponse<ExecC>,
    ) -> AnyResult<AppIbcBasicResponse> {
        // We format the events correctly because we are executing wasm
        let contract_response = Response::new()
            .add_submessages(res.messages)
            .add_attributes(res.attributes)
            .add_events(res.events);

        let (res, msgs) = self.build_app_response(&contract, Event::new("ibc"), contract_response);

        // We process eventual messages that were sent out with the response
        let res = self.process_response(api, router, storage, block, contract, res, msgs)?;

        // We transfer back to an IbcBasicResponse
        Ok(AppIbcBasicResponse { events: res.events })
    }

    fn process_ibc_receive_response(
        &self,
        api: &dyn Api,
        contract: Addr,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        original_res: IbcReceiveResponse<ExecC>,
    ) -> AnyResult<AppIbcReceiveResponse> {
        // We format the events correctly because we are executing wasm
        let contract_response = Response::new()
            .add_submessages(original_res.messages)
            .add_attributes(original_res.attributes)
            .add_events(original_res.events);

        let (res, msgs) = self.build_app_response(&contract, Event::new("ibc"), contract_response);

        // We process eventual messages that were sent out with the response
        let res = self.process_response(api, router, storage, block, contract, res, msgs)?;

        // If the data field was overwritten by the response propagation, we replace the ibc ack
        let acknowledgement = res.data.or(original_res.acknowledgement);

        // We transfer back to an IbcBasicResponse
        Ok(AppIbcReceiveResponse {
            events: res.events,
            acknowledgement,
        })
    }

    /// Creates a contract address and empty storage instance.
    /// Returns the new contract address.
    ///
    /// You have to call init after this to set up the contract properly.
    /// These two steps are separated to have cleaner return values.
    pub fn register_contract(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        code_id: u64,
        creator: Addr,
        admin: impl Into<Option<Addr>>,
        label: String,
        created: u64,
        salt: impl Into<Option<Binary>>,
    ) -> AnyResult<Addr> {
        // check if the contract's code with specified code_id exists
        if code_id as usize > self.code_data.len() {
            bail!("Cannot init contract with unregistered code id");
        }

        // generate a new contract address
        let instance_id = self.instance_count(storage) as u64;
        let addr = if let Some(salt_binary) = salt.into() {
            // generate predictable contract address when salt is provided
            let code_data = self.code_data(code_id)?;
            let canonical_addr = &api.addr_canonicalize(creator.as_ref())?;
            self.address_generator.predictable_contract_address(
                api,
                storage,
                code_id,
                instance_id,
                code_data.checksum.as_slice(),
                canonical_addr,
                salt_binary.as_slice(),
            )?
        } else {
            // generate non-predictable contract address
            self.address_generator
                .contract_address(api, storage, code_id, instance_id)?
        };

        // contract with the same address must not already exist
        if self.contract_data(storage, &addr).is_ok() {
            bail!(Error::duplicated_contract_address(addr));
        }

        // prepare contract data and save new contract instance
        let info = ContractData {
            code_id,
            creator,
            admin: admin.into(),
            label,
            created,
        };
        self.save_contract(storage, &addr, &info)?;
        Ok(addr)
    }

    /// Executes contract's `execute` entry-point.
    pub fn call_execute(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        address: Addr,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<ExecC>> {
        Self::verify_response(self.with_storage(
            api,
            storage,
            router,
            block,
            address,
            |contract, deps, env| contract.execute(deps, env, info, msg),
        )?)
    }

    /// Executes contract's `instantiate` entry-point.
    pub fn call_instantiate(
        &self,
        address: Addr,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<ExecC>> {
        Self::verify_response(self.with_storage(
            api,
            storage,
            router,
            block,
            address,
            |contract, deps, env| contract.instantiate(deps, env, info, msg),
        )?)
    }

    /// Executes contract's `reply` entry-point.
    pub fn call_reply(
        &self,
        address: Addr,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        reply: Reply,
    ) -> AnyResult<Response<ExecC>> {
        Self::verify_response(self.with_storage(
            api,
            storage,
            router,
            block,
            address,
            |contract, deps, env| contract.reply(deps, env, reply),
        )?)
    }

    /// Executes contract's `sudo` entry-point.
    pub fn call_sudo(
        &self,
        address: Addr,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<ExecC>> {
        Self::verify_response(self.with_storage(
            api,
            storage,
            router,
            block,
            address,
            |contract, deps, env| contract.sudo(deps, env, msg),
        )?)
    }

    /// Executes contract's `migrate` entry-point.
    pub fn call_migrate(
        &self,
        address: Addr,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<ExecC>> {
        Self::verify_response(self.with_storage(
            api,
            storage,
            router,
            block,
            address,
            |contract, deps, env| contract.migrate(deps, env, msg),
        )?)
    }

    fn get_env<T: Into<Addr>>(&self, address: T, block: &BlockInfo) -> Env {
        Env {
            block: block.clone(),
            contract: ContractInfo {
                address: address.into(),
            },
            transaction: Some(TransactionInfo { index: 0 }),
        }
    }

    fn with_storage_readonly<F, T>(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        querier: &dyn Querier,
        block: &BlockInfo,
        address: Addr,
        action: F,
    ) -> AnyResult<T>
    where
        F: FnOnce(&dyn Contract<ExecC, QueryC>, Deps<QueryC>, Env) -> AnyResult<T>,
    {
        let contract = self.contract_data(storage, &address)?;
        let handler = self.contract_code(contract.code_id)?;
        let storage = self.contract_storage(storage, &address);
        let env = self.get_env(address, block);

        let deps = Deps {
            storage: storage.as_ref(),
            api,
            querier: QuerierWrapper::new(querier),
        };
        action(handler, deps, env)
    }

    fn with_storage<F, T>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        address: Addr,
        action: F,
    ) -> AnyResult<T>
    where
        F: FnOnce(&dyn Contract<ExecC, QueryC>, DepsMut<QueryC>, Env) -> AnyResult<T>,
        ExecC: DeserializeOwned,
    {
        let contract = self.contract_data(storage, &address)?;
        let handler = self.contract_code(contract.code_id)?;

        // We don't actually need a transaction here, as it is already embedded in a transactional.
        // execute_submsg or App.execute_multi.
        // However, we need to get write and read access to the same storage in two different objects,
        // and this is the only way I know how to do so.
        transactional(storage, |write_cache, read_store| {
            let mut contract_storage = self.contract_storage_mut(write_cache, &address);
            let querier = RouterQuerier::new(router, api, read_store, block);
            let env = self.get_env(address, block);

            let deps = DepsMut {
                storage: contract_storage.as_mut(),
                api,
                querier: QuerierWrapper::new(&querier),
            };
            action(handler, deps, env)
        })
    }

    /// Saves contract data in a storage under specified address.
    pub fn save_contract(
        &self,
        storage: &mut dyn Storage,
        address: &Addr,
        contract: &ContractData,
    ) -> AnyResult<()> {
        CONTRACTS
            .save(&mut prefixed(storage, NAMESPACE_WASM), address, contract)
            .map_err(Into::into)
    }

    /// Returns the number of all contract instances.
    fn instance_count(&self, storage: &dyn Storage) -> usize {
        CONTRACTS
            .range_raw(
                &prefixed_read(storage, NAMESPACE_WASM),
                None,
                None,
                Order::Ascending,
            )
            .count()
    }

    /// Returns the response type for specified message.
    fn response_type_url(msg: &CosmosMsg<ExecC>) -> String {
        const UNKNOWN: &str = "/unknown";
        match &msg {
            CosmosMsg::Bank(bank_msg) => match bank_msg {
                BankMsg::Send { .. } => "/cosmos.bank.v1beta1.MsgSendResponse",
                BankMsg::Burn { .. } => "/cosmos.bank.v1beta1.MsgBurnResponse",
                _ => UNKNOWN,
            },
            CosmosMsg::Custom(..) => UNKNOWN,
            #[cfg(feature = "staking")]
            CosmosMsg::Staking(staking_msg) => match staking_msg {
                StakingMsg::Delegate { .. } => "/cosmos.staking.v1beta1.MsgDelegateResponse",
                StakingMsg::Undelegate { .. } => "/cosmos.staking.v1beta1.MsgUndelegateResponse",
                StakingMsg::Redelegate { .. } => {
                    "/cosmos.staking.v1beta1.MsgBeginRedelegateResponse"
                }
                _ => UNKNOWN,
            },
            #[cfg(feature = "staking")]
            CosmosMsg::Distribution(distribution_msg) => match distribution_msg {
                #[cfg(feature = "cosmwasm_1_3")]
                DistributionMsg::FundCommunityPool { .. } => {
                    "/cosmos.distribution.v1beta1.MsgFundCommunityPoolResponse"
                }
                DistributionMsg::SetWithdrawAddress { .. } => {
                    "/cosmos.distribution.v1beta1.MsgSetWithdrawAddressResponse"
                }
                DistributionMsg::WithdrawDelegatorReward { .. } => {
                    "/cosmos.distribution.v1beta1.MsgWithdrawDelegatorRewardResponse"
                }
                _ => UNKNOWN,
            },
            #[cfg(feature = "stargate")]
            #[allow(deprecated)]
            CosmosMsg::Stargate { .. } => UNKNOWN,
            #[cfg(feature = "cosmwasm_2_0")]
            CosmosMsg::Any(..) => UNKNOWN,
            #[cfg(feature = "stargate")]
            CosmosMsg::Ibc(..) => UNKNOWN,
            CosmosMsg::Wasm(wasm_msg) => match wasm_msg {
                WasmMsg::Instantiate { .. } => "/cosmwasm.wasm.v1.MsgInstantiateContractResponse",
                #[cfg(feature = "cosmwasm_1_2")]
                WasmMsg::Instantiate2 { .. } => "/cosmwasm.wasm.v1.MsgInstantiateContract2Response",
                WasmMsg::Execute { .. } => "/cosmwasm.wasm.v1.MsgExecuteContractResponse",
                WasmMsg::Migrate { .. } => "/cosmwasm.wasm.v1.MsgMigrateContractResponse",
                WasmMsg::UpdateAdmin { .. } => "/cosmwasm.wasm.v1.MsgUpdateAdminResponse",
                WasmMsg::ClearAdmin { .. } => "/cosmwasm.wasm.v1.MsgClearAdminResponse",
                _ => UNKNOWN,
            },
            #[cfg(feature = "stargate")]
            CosmosMsg::Gov(gov_msg) => match gov_msg {
                GovMsg::Vote { .. } => "/cosmos.gov.v1beta1.MsgVoteResponse",
                #[cfg(feature = "cosmwasm_1_2")]
                GovMsg::VoteWeighted { .. } => "/cosmos.gov.v1beta1.MsgVoteWeightedResponse",
            },
            _ => UNKNOWN,
        }
        .to_string()
    }
}

#[derive(Clone, PartialEq, Message)]
struct InstantiateResponse {
    #[prost(string, tag = "1")]
    pub address: String,
    #[prost(bytes, tag = "2")]
    pub data: Vec<u8>,
}

fn instantiate_response(data: Option<Binary>, contact_address: &Addr) -> Binary {
    let data = data.unwrap_or_default().to_vec();
    let init_data = InstantiateResponse {
        address: contact_address.into(),
        data,
    };
    let mut new_data = Vec::<u8>::with_capacity(init_data.encoded_len());
    // the data must encode successfully
    init_data.encode(&mut new_data).unwrap();
    new_data.into()
}

#[derive(Clone, PartialEq, Message)]
struct ExecuteResponse {
    #[prost(bytes, tag = "1")]
    pub data: Vec<u8>,
}

/// Encodes the response data.
fn encode_response_data(data: Option<Binary>) -> Option<Binary> {
    data.map(|d| {
        let execute_response = ExecuteResponse { data: d.to_vec() };
        let mut encoded_data = Vec::<u8>::with_capacity(execute_response.encoded_len());
        execute_response.encode(&mut encoded_data).unwrap();
        encoded_data.into()
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::app::Router;
    use crate::bank::BankKeeper;
    use crate::featured::staking::{DistributionKeeper, StakeKeeper};
    use crate::module::FailingModule;
    use crate::test_helpers::{caller, error, payout};
    use crate::transactions::StorageTransaction;
    use crate::{GovFailingModule, IbcFailingModule, StargateFailing};
    use cosmwasm_std::testing::{message_info, mock_env, MockApi, MockQuerier, MockStorage};
    #[cfg(feature = "cosmwasm_1_2")]
    use cosmwasm_std::CodeInfoResponse;
    use cosmwasm_std::{
        coin, from_json, to_json_vec, CanonicalAddr, CosmosMsg, Empty, HexBinary, StdError,
    };

    /// Type alias for default build `Router` to make its reference in typical scenario
    type BasicRouter<ExecC = Empty, QueryC = Empty> = Router<
        BankKeeper,
        FailingModule<ExecC, QueryC, Empty>,
        WasmKeeper<ExecC, QueryC>,
        StakeKeeper,
        DistributionKeeper,
        IbcFailingModule,
        GovFailingModule,
        StargateFailing,
    >;

    fn wasm_keeper() -> WasmKeeper<Empty, Empty> {
        WasmKeeper::new()
    }

    fn mock_router() -> BasicRouter {
        Router {
            wasm: WasmKeeper::new(),
            bank: BankKeeper::new(),
            custom: FailingModule::new(),
            staking: StakeKeeper::new(),
            distribution: DistributionKeeper::new(),
            ibc: IbcFailingModule::new(),
            gov: GovFailingModule::new(),
            stargate: StargateFailing,
        }
    }

    #[test]
    fn register_contract() {
        let api = MockApi::default();

        // prepare user addresses
        let creator_addr = api.addr_make("creator");
        let user_addr = api.addr_make("foobar");
        let admin_addr = api.addr_make("admin");
        let unregistered_addr = api.addr_make("unregistered");

        let mut wasm_storage = MockStorage::new();
        let mut wasm_keeper = wasm_keeper();
        let block = mock_env().block;
        let code_id = wasm_keeper.store_code(creator_addr, error::contract(false));

        transactional(&mut wasm_storage, |cache, _| {
            // cannot register contract with unregistered codeId
            wasm_keeper.register_contract(
                &api,
                cache,
                code_id + 1,
                user_addr.clone(),
                admin_addr.clone(),
                "label".to_owned(),
                1000,
                None,
            )
        })
        .unwrap_err();

        let contract_addr = transactional(&mut wasm_storage, |cache, _| {
            // we can register a new instance of this code
            wasm_keeper.register_contract(
                &api,
                cache,
                code_id,
                user_addr.clone(),
                admin_addr.clone(),
                "label".to_owned(),
                1000,
                None,
            )
        })
        .unwrap();

        // verify contract data are as expected
        let contract_data = wasm_keeper
            .contract_data(&wasm_storage, &contract_addr)
            .unwrap();

        assert_eq!(
            contract_data,
            ContractData {
                code_id,
                creator: user_addr.clone(),
                admin: admin_addr.into(),
                label: "label".to_owned(),
                created: 1000,
            }
        );

        let err = transactional(&mut wasm_storage, |cache, _| {
            // now, we call this contract and see the error message from the contract
            let info = message_info(&user_addr, &[]);
            wasm_keeper.call_instantiate(
                contract_addr.clone(),
                &api,
                cache,
                &mock_router(),
                &block,
                info,
                b"{}".to_vec(),
            )
        })
        .unwrap_err();

        // StdError from contract_error auto-converted to string
        assert_eq!(
            StdError::generic_err("Init failed"),
            err.downcast().unwrap()
        );

        let err = transactional(&mut wasm_storage, |cache, _| {
            // and the error for calling an unregistered contract
            let info = message_info(&user_addr, &[]);
            wasm_keeper.call_instantiate(
                unregistered_addr,
                &api,
                cache,
                &mock_router(),
                &block,
                info,
                b"{}".to_vec(),
            )
        })
        .unwrap_err();

        // Default error message from router when not found
        assert!(matches!(err.downcast().unwrap(), StdError::NotFound { .. }));
    }

    #[test]
    fn query_contract_info() {
        let api = MockApi::default();

        // prepare user addresses
        let creator_addr = api.addr_make("creator");
        let admin_addr = api.addr_make("admin");

        let mut wasm_storage = MockStorage::new();
        let mut wasm_keeper = wasm_keeper();
        let block = mock_env().block;
        let code_id = wasm_keeper.store_code(creator_addr.clone(), payout::contract());
        assert_eq!(1, code_id);

        let contract_addr = wasm_keeper
            .register_contract(
                &api,
                &mut wasm_storage,
                code_id,
                creator_addr.clone(),
                admin_addr.clone(),
                "label".to_owned(),
                1000,
                None,
            )
            .unwrap();

        let querier: MockQuerier<Empty> = MockQuerier::new(&[]);
        let query = WasmQuery::ContractInfo {
            contract_addr: contract_addr.into(),
        };

        let contract_info = wasm_keeper
            .query(&api, &wasm_storage, &querier, &block, query)
            .unwrap();

        let actual: ContractInfoResponse = from_json(contract_info).unwrap();
        let expected =
            ContractInfoResponse::new(code_id, creator_addr, admin_addr.into(), false, None);
        assert_eq!(expected, actual);
    }

    #[test]
    #[cfg(feature = "cosmwasm_1_2")]
    fn query_code_info() {
        let api = MockApi::default();
        let wasm_storage = MockStorage::new();
        let mut wasm_keeper = wasm_keeper();
        let block = mock_env().block;
        let creator_addr = api.addr_make("creator");
        let code_id = wasm_keeper.store_code(creator_addr.clone(), payout::contract());
        let querier: MockQuerier<Empty> = MockQuerier::new(&[]);
        let query = WasmQuery::CodeInfo { code_id };
        let code_info = wasm_keeper
            .query(&api, &wasm_storage, &querier, &block, query)
            .unwrap();
        let actual: CodeInfoResponse = from_json(code_info).unwrap();
        assert_eq!(code_id, actual.code_id);
        assert_eq!(creator_addr.as_str(), actual.creator.as_str());
        assert_eq!(32, actual.checksum.as_slice().len());
    }

    #[test]
    #[cfg(feature = "cosmwasm_1_2")]
    fn different_contracts_must_have_different_checksum() {
        let api = MockApi::default();
        let creator_addr = api.addr_make("creator");
        let wasm_storage = MockStorage::new();
        let mut wasm_keeper = wasm_keeper();
        let block = mock_env().block;
        let code_id_payout = wasm_keeper.store_code(creator_addr.clone(), payout::contract());
        let code_id_caller = wasm_keeper.store_code(creator_addr, caller::contract());
        let querier: MockQuerier<Empty> = MockQuerier::new(&[]);
        let query_payout = WasmQuery::CodeInfo {
            code_id: code_id_payout,
        };
        let query_caller = WasmQuery::CodeInfo {
            code_id: code_id_caller,
        };
        let code_info_payout = wasm_keeper
            .query(&api, &wasm_storage, &querier, &block, query_payout)
            .unwrap();
        let code_info_caller = wasm_keeper
            .query(&api, &wasm_storage, &querier, &block, query_caller)
            .unwrap();
        let info_payout: CodeInfoResponse = from_json(code_info_payout).unwrap();
        let info_caller: CodeInfoResponse = from_json(code_info_caller).unwrap();
        assert_eq!(code_id_payout, info_payout.code_id);
        assert_eq!(code_id_caller, info_caller.code_id);
        assert_ne!(info_caller.code_id, info_payout.code_id);
        assert_eq!(info_caller.creator, info_payout.creator);
        assert_ne!(info_caller.checksum, info_payout.checksum);
    }

    #[test]
    #[cfg(feature = "cosmwasm_1_2")]
    fn querying_invalid_code_info_must_fail() {
        let api = MockApi::default();
        let wasm_storage = MockStorage::new();
        let wasm_keeper = wasm_keeper();
        let block = mock_env().block;

        let querier: MockQuerier<Empty> = MockQuerier::new(&[]);
        let query = WasmQuery::CodeInfo { code_id: 100 };

        wasm_keeper
            .query(&api, &wasm_storage, &querier, &block, query)
            .unwrap_err();
    }

    #[test]
    fn can_dump_raw_wasm_state() {
        let api = MockApi::default();

        // prepare user addresses
        let creator_addr = api.addr_make("creator");
        let admin_addr = api.addr_make("admin");
        let user_addr = api.addr_make("foobar");

        let mut wasm_keeper = wasm_keeper();
        let block = mock_env().block;
        let code_id = wasm_keeper.store_code(creator_addr, payout::contract());

        let mut wasm_storage = MockStorage::new();

        let contract_addr = wasm_keeper
            .register_contract(
                &api,
                &mut wasm_storage,
                code_id,
                user_addr.clone(),
                admin_addr,
                "label".to_owned(),
                1000,
                None,
            )
            .unwrap();

        // make a contract with state
        let payout = coin(1500, "mlg");
        let msg = payout::InstantiateMessage {
            payout: payout.clone(),
        };
        wasm_keeper
            .call_instantiate(
                contract_addr.clone(),
                &api,
                &mut wasm_storage,
                &mock_router(),
                &block,
                message_info(&user_addr, &[]),
                to_json_vec(&msg).unwrap(),
            )
            .unwrap();

        // dump state
        let state = wasm_keeper.dump_wasm_raw(&wasm_storage, &contract_addr);
        assert_eq!(state.len(), 2);
        // check contents
        let (k, v) = &state[0];
        assert_eq!(k.as_slice(), b"count");
        let count: u32 = from_json(v).unwrap();
        assert_eq!(count, 1);
        let (k, v) = &state[1];
        assert_eq!(k.as_slice(), b"payout");
        let stored_pay: payout::InstantiateMessage = from_json(v).unwrap();
        assert_eq!(stored_pay.payout, payout);
    }

    #[test]
    fn contract_send_coins() {
        let api = MockApi::default();

        // prepare user addresses
        let creator_addr = api.addr_make("creator");
        let user_addr = api.addr_make("foobar");

        let mut wasm_keeper = wasm_keeper();
        let block = mock_env().block;
        let code_id = wasm_keeper.store_code(creator_addr, payout::contract());

        let mut wasm_storage = MockStorage::new();
        let mut cache = StorageTransaction::new(&wasm_storage);

        let contract_addr = wasm_keeper
            .register_contract(
                &api,
                &mut cache,
                code_id,
                user_addr.clone(),
                None,
                "label".to_owned(),
                1000,
                None,
            )
            .unwrap();

        let payout = coin(100, "TGD");

        // init the contract
        let info = message_info(&user_addr, &[]);
        let init_msg = to_json_vec(&payout::InstantiateMessage {
            payout: payout.clone(),
        })
        .unwrap();
        let res = wasm_keeper
            .call_instantiate(
                contract_addr.clone(),
                &api,
                &mut cache,
                &mock_router(),
                &block,
                info,
                init_msg,
            )
            .unwrap();
        assert_eq!(0, res.messages.len());

        // execute the contract
        let info = message_info(&user_addr, &[]);
        let res = wasm_keeper
            .call_execute(
                &api,
                &mut cache,
                contract_addr.clone(),
                &mock_router(),
                &block,
                info,
                b"{}".to_vec(),
            )
            .unwrap();
        assert_eq!(1, res.messages.len());
        match &res.messages[0].msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(to_address.as_str(), user_addr.as_str());
                assert_eq!(amount.as_slice(), &[payout.clone()]);
            }
            m => panic!("Unexpected message {:?}", m),
        }

        // and flush before query
        cache.prepare().commit(&mut wasm_storage);

        // query the contract
        let query = to_json_vec(&payout::QueryMsg::Payout {}).unwrap();
        let querier: MockQuerier<Empty> = MockQuerier::new(&[]);
        let data = wasm_keeper
            .query_smart(contract_addr, &api, &wasm_storage, &querier, &block, query)
            .unwrap();
        let res: payout::InstantiateMessage = from_json(data).unwrap();
        assert_eq!(res.payout, payout);
    }

    fn assert_payout(
        router: &WasmKeeper<Empty, Empty>,
        storage: &mut dyn Storage,
        contract_addr: &Addr,
        payout: &Coin,
    ) {
        let api = MockApi::default();
        let user_addr = api.addr_make("silly");
        let info = message_info(&user_addr, &[]);
        let res = router
            .call_execute(
                &api,
                storage,
                contract_addr.clone(),
                &mock_router(),
                &mock_env().block,
                info,
                b"{}".to_vec(),
            )
            .unwrap();
        assert_eq!(1, res.messages.len());
        match &res.messages[0].msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(to_address.as_str(), user_addr.as_str());
                assert_eq!(amount.as_slice(), &[payout.clone()]);
            }
            m => panic!("Unexpected message {:?}", m),
        }
    }

    fn assert_no_contract(storage: &dyn Storage, contract_addr: &Addr) {
        let contract = CONTRACTS.may_load(storage, contract_addr).unwrap();
        assert!(contract.is_none(), "{:?}", contract_addr);
    }

    #[test]
    fn multi_level_wasm_cache() {
        let api = MockApi::default();

        // prepare user addresses
        let creator_addr = api.addr_make("creator");
        let user_addr = api.addr_make("foobar");
        let user_addr_1 = api.addr_make("johnny");

        let mut wasm_keeper = wasm_keeper();
        let block = mock_env().block;
        let code_id = wasm_keeper.store_code(creator_addr, payout::contract());

        let mut wasm_storage = MockStorage::new();

        let payout1 = coin(100, "TGD");

        // set contract 1 and commit (on router)
        let contract1 = transactional(&mut wasm_storage, |cache, _| {
            let contract = wasm_keeper
                .register_contract(
                    &api,
                    cache,
                    code_id,
                    user_addr.clone(),
                    None,
                    "".to_string(),
                    1000,
                    None,
                )
                .unwrap();
            let info = message_info(&user_addr, &[]);
            let init_msg = to_json_vec(&payout::InstantiateMessage {
                payout: payout1.clone(),
            })
            .unwrap();
            wasm_keeper
                .call_instantiate(
                    contract.clone(),
                    &api,
                    cache,
                    &mock_router(),
                    &block,
                    info,
                    init_msg,
                )
                .unwrap();

            Ok(contract)
        })
        .unwrap();

        let payout2 = coin(50, "BTC");
        let payout3 = coin(1234, "ATOM");

        // create a new cache and check we can use contract 1
        let (contract2, contract3) = transactional(&mut wasm_storage, |cache, wasm_reader| {
            assert_payout(&wasm_keeper, cache, &contract1, &payout1);

            // create contract 2 and use it
            let contract2 = wasm_keeper
                .register_contract(
                    &api,
                    cache,
                    code_id,
                    user_addr.clone(),
                    None,
                    "".to_owned(),
                    1000,
                    None,
                )
                .unwrap();
            let info = message_info(&user_addr, &[]);
            let init_msg = to_json_vec(&payout::InstantiateMessage {
                payout: payout2.clone(),
            })
            .unwrap();
            let _res = wasm_keeper
                .call_instantiate(
                    contract2.clone(),
                    &api,
                    cache,
                    &mock_router(),
                    &block,
                    info,
                    init_msg,
                )
                .unwrap();
            assert_payout(&wasm_keeper, cache, &contract2, &payout2);

            // create a level2 cache and check we can use contract 1 and contract 2
            let contract3 = transactional(cache, |cache2, read| {
                assert_payout(&wasm_keeper, cache2, &contract1, &payout1);
                assert_payout(&wasm_keeper, cache2, &contract2, &payout2);

                // create a contract on level 2
                let contract3 = wasm_keeper
                    .register_contract(
                        &api,
                        cache2,
                        code_id,
                        user_addr,
                        None,
                        "".to_owned(),
                        1000,
                        None,
                    )
                    .unwrap();
                let info = message_info(&user_addr_1, &[]);
                let init_msg = to_json_vec(&payout::InstantiateMessage {
                    payout: payout3.clone(),
                })
                .unwrap();
                let _res = wasm_keeper
                    .call_instantiate(
                        contract3.clone(),
                        &api,
                        cache2,
                        &mock_router(),
                        &block,
                        info,
                        init_msg,
                    )
                    .unwrap();
                assert_payout(&wasm_keeper, cache2, &contract3, &payout3);

                // ensure first cache still doesn't see this contract
                assert_no_contract(read, &contract3);
                Ok(contract3)
            })
            .unwrap();

            // after applying transaction, all contracts present on cache
            assert_payout(&wasm_keeper, cache, &contract1, &payout1);
            assert_payout(&wasm_keeper, cache, &contract2, &payout2);
            assert_payout(&wasm_keeper, cache, &contract3, &payout3);

            // but not yet the root router
            assert_no_contract(wasm_reader, &contract1);
            assert_no_contract(wasm_reader, &contract2);
            assert_no_contract(wasm_reader, &contract3);

            Ok((contract2, contract3))
        })
        .unwrap();

        // ensure that it is now applied to the router
        assert_payout(&wasm_keeper, &mut wasm_storage, &contract1, &payout1);
        assert_payout(&wasm_keeper, &mut wasm_storage, &contract2, &payout2);
        assert_payout(&wasm_keeper, &mut wasm_storage, &contract3, &payout3);
    }

    fn assert_admin(
        storage: &dyn Storage,
        wasm_keeper: &WasmKeeper<Empty, Empty>,
        contract_addr: &impl ToString,
        admin: Option<Addr>,
    ) {
        let api = MockApi::default();
        let querier: MockQuerier<Empty> = MockQuerier::new(&[]);
        // query
        let data = wasm_keeper
            .query(
                &api,
                storage,
                &querier,
                &mock_env().block,
                WasmQuery::ContractInfo {
                    contract_addr: contract_addr.to_string(),
                },
            )
            .unwrap();
        let res: ContractInfoResponse = from_json(data).unwrap();
        assert_eq!(res.admin, admin);
    }

    #[test]
    fn update_clear_admin_works() {
        let api = MockApi::default();
        let mut wasm_keeper = wasm_keeper();
        let block = mock_env().block;
        let creator = api.addr_make("creator");
        let code_id = wasm_keeper.store_code(creator.clone(), caller::contract());

        let mut wasm_storage = MockStorage::new();

        let admin = api.addr_make("admin");
        let new_admin = api.addr_make("new_admin");
        let normal_user = api.addr_make("normal_user");

        let contract_addr = wasm_keeper
            .register_contract(
                &api,
                &mut wasm_storage,
                code_id,
                creator,
                admin.clone(),
                "label".to_owned(),
                1000,
                None,
            )
            .unwrap();

        // init the contract
        let info = message_info(&admin, &[]);
        let init_msg = to_json_vec(&Empty {}).unwrap();
        let res = wasm_keeper
            .call_instantiate(
                contract_addr.clone(),
                &api,
                &mut wasm_storage,
                &mock_router(),
                &block,
                info,
                init_msg,
            )
            .unwrap();
        assert_eq!(0, res.messages.len());

        assert_admin(
            &wasm_storage,
            &wasm_keeper,
            &contract_addr,
            Some(admin.clone()),
        );

        // non-admin should not be allowed to become admin on their own
        wasm_keeper
            .execute_wasm(
                &api,
                &mut wasm_storage,
                &mock_router(),
                &block,
                normal_user.clone(),
                WasmMsg::UpdateAdmin {
                    contract_addr: contract_addr.to_string(),
                    admin: normal_user.to_string(),
                },
            )
            .unwrap_err();

        // should still be admin
        assert_admin(
            &wasm_storage,
            &wasm_keeper,
            &contract_addr,
            Some(admin.clone()),
        );

        // admin should be allowed to transfer administration permissions
        let res = wasm_keeper
            .execute_wasm(
                &api,
                &mut wasm_storage,
                &mock_router(),
                &block,
                admin,
                WasmMsg::UpdateAdmin {
                    contract_addr: contract_addr.to_string(),
                    admin: new_admin.to_string(),
                },
            )
            .unwrap();
        assert_eq!(res.events.len(), 0);

        // new_admin should now be admin
        assert_admin(
            &wasm_storage,
            &wasm_keeper,
            &contract_addr,
            Some(new_admin.clone()),
        );

        // new_admin should now be able to clear to admin
        let res = wasm_keeper
            .execute_wasm(
                &api,
                &mut wasm_storage,
                &mock_router(),
                &block,
                new_admin,
                WasmMsg::ClearAdmin {
                    contract_addr: contract_addr.to_string(),
                },
            )
            .unwrap();
        assert_eq!(res.events.len(), 0);

        // should have no admin now
        assert_admin(&wasm_storage, &wasm_keeper, &contract_addr, None);
    }

    #[test]
    fn uses_simple_address_generator_by_default() {
        let api = MockApi::default();
        let mut wasm_keeper = wasm_keeper();
        let creator_addr = api.addr_make("creator");
        let code_id = wasm_keeper.store_code(creator_addr.clone(), payout::contract());
        assert_eq!(1, code_id);

        let mut wasm_storage = MockStorage::new();

        let admin = api.addr_make("admin");
        let contract_addr = wasm_keeper
            .register_contract(
                &api,
                &mut wasm_storage,
                code_id,
                creator_addr.clone(),
                admin.clone(),
                "label".to_owned(),
                1000,
                None,
            )
            .unwrap();

        assert_eq!(
            contract_addr.as_str(),
            "cosmwasm1mzdhwvvh22wrt07w59wxyd58822qavwkx5lcej7aqfkpqqlhaqfsgn6fq2",
            "default address generator returned incorrect address"
        );

        let salt = HexBinary::from_hex("c0ffee").unwrap();

        let contract_addr = wasm_keeper
            .register_contract(
                &api,
                &mut wasm_storage,
                code_id,
                creator_addr.clone(),
                admin.clone(),
                "label".to_owned(),
                1000,
                Binary::from(salt.clone()),
            )
            .unwrap();

        assert_eq!(
            contract_addr.as_str(),
            "cosmwasm1drhu6t78wacgm5qjzs4hvkv9fd9awa9henw7fh6vmzrhf7k2nkjsg3flns",
            "default address generator returned incorrect address"
        );

        let code_id = wasm_keeper.store_code(creator_addr, payout::contract());
        assert_eq!(2, code_id);

        let user_addr = api.addr_make("boobaz");

        let contract_addr = wasm_keeper
            .register_contract(
                &api,
                &mut wasm_storage,
                code_id,
                user_addr,
                admin,
                "label".to_owned(),
                1000,
                Binary::from(salt),
            )
            .unwrap();

        assert_eq!(
            contract_addr.as_str(),
            "cosmwasm13cfeertf2gny0rzp5jwqzst8crmfgvcd2lq5su0c9z66yxa45qdsdd0uxc",
            "default address generator returned incorrect address"
        );
    }

    struct TestAddressGenerator {
        address: Addr,
        predictable_address: Addr,
    }

    impl AddressGenerator for TestAddressGenerator {
        fn contract_address(
            &self,
            _api: &dyn Api,
            _storage: &mut dyn Storage,
            _code_id: u64,
            _instance_id: u64,
        ) -> AnyResult<Addr> {
            Ok(self.address.clone())
        }

        fn predictable_contract_address(
            &self,
            _api: &dyn Api,
            _storage: &mut dyn Storage,
            _code_id: u64,
            _instance_id: u64,
            _checksum: &[u8],
            _creator: &CanonicalAddr,
            _salt: &[u8],
        ) -> AnyResult<Addr> {
            Ok(self.predictable_address.clone())
        }
    }

    #[test]
    fn can_use_custom_address_generator() {
        let api = MockApi::default();
        let expected_addr = api.addr_make("address");
        let expected_predictable_addr = api.addr_make("predictable_address");
        let mut wasm_keeper: WasmKeeper<Empty, Empty> =
            WasmKeeper::new().with_address_generator(TestAddressGenerator {
                address: expected_addr.clone(),
                predictable_address: expected_predictable_addr.clone(),
            });
        let creator = api.addr_make("creator");
        let code_id = wasm_keeper.store_code(creator.clone(), payout::contract());

        let mut wasm_storage = MockStorage::new();

        let admin = api.addr_make("admin");
        let contract_addr = wasm_keeper
            .register_contract(
                &api,
                &mut wasm_storage,
                code_id,
                creator.clone(),
                admin.clone(),
                "label".to_owned(),
                1000,
                None,
            )
            .unwrap();

        assert_eq!(
            contract_addr, expected_addr,
            "custom address generator returned incorrect address"
        );

        let contract_addr = wasm_keeper
            .register_contract(
                &api,
                &mut wasm_storage,
                code_id,
                creator,
                admin,
                "label".to_owned(),
                1000,
                Binary::from(HexBinary::from_hex("23A74B8C").unwrap()),
            )
            .unwrap();

        assert_eq!(
            contract_addr, expected_predictable_addr,
            "custom address generator returned incorrect address"
        );
    }
}
