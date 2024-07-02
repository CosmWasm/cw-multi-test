use crate::bank::{Bank, BankKeeper, BankSudo};
use crate::contracts::Contract;
use crate::error::{bail, AnyResult};
use crate::executor::{AppResponse, Executor};
use crate::gov::Gov;
use crate::ibc::{
    types::IbcResponse, types::MockIbcQuery, IbcModuleMsg, IbcPacketRelayingMsg as IbcSudo,
};
use crate::ibc::{Ibc, IbcSimpleModule};
use crate::module::{FailingModule, Module};
use crate::prefixed_storage::{
    prefixed, prefixed_multilevel, prefixed_multilevel_read, prefixed_read,
};
use crate::staking::{Distribution, DistributionKeeper, StakeKeeper, Staking, StakingSudo};
use crate::transactions::transactional;
use crate::wasm::{ContractData, Wasm, WasmKeeper, WasmSudo};
use crate::{AppBuilder, GovFailingModule, Stargate, StargateFailing};
use cosmwasm_std::testing::{MockApi, MockStorage};
use cosmwasm_std::{
    from_json, to_json_binary, Addr, Api, Binary, BlockInfo, ContractResult, CosmosMsg, CustomMsg,
    CustomQuery, Empty, Querier, QuerierResult, QuerierWrapper, QueryRequest, Record, Storage,
    SystemError, SystemResult,
};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use std::marker::PhantomData;

/// Advances the blockchain environment to the next block in tests, enabling developers to simulate
/// time-dependent contract behaviors and block-related triggers efficiently.
pub fn next_block(block: &mut BlockInfo) {
    block.time = block.time.plus_seconds(5);
    block.height += 1;
}

/// A type alias for the default-built App. It simplifies storage and handling in typical scenarios,
/// streamlining the use of the App structure in standard test setups.
pub type BasicApp<ExecC = Empty, QueryC = Empty> = App<
    BankKeeper,
    MockApi,
    MockStorage,
    FailingModule<ExecC, QueryC, Empty>,
    WasmKeeper<ExecC, QueryC>,
    StakeKeeper,
    DistributionKeeper,
    IbcSimpleModule,
    GovFailingModule,
    StargateFailing,
>;

/// # Blockchain application simulator
///
/// This structure is the main component of the real-life blockchain simulator.
#[derive(Clone)]
pub struct App<
    Bank = BankKeeper,
    Api = MockApi,
    Storage = MockStorage,
    Custom = FailingModule<Empty, Empty, Empty>,
    Wasm = WasmKeeper<Empty, Empty>,
    Staking = StakeKeeper,
    Distr = DistributionKeeper,
    Ibc = IbcSimpleModule,
    Gov = GovFailingModule,
    Stargate = StargateFailing,
> {
    pub(crate) router: Router<Bank, Custom, Wasm, Staking, Distr, Ibc, Gov, Stargate>,
    pub(crate) api: Api,
    pub(crate) storage: Storage,
    pub(crate) block: BlockInfo,
}

/// No-op application initialization function.
pub fn no_init<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>(
    router: &mut Router<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>,
    api: &dyn Api,
    storage: &mut dyn Storage,
) {
    let _ = (router, api, storage);
}

impl Default for BasicApp {
    fn default() -> Self {
        Self::new(no_init)
    }
}

impl BasicApp {
    /// Creates new default `App` implementation working with Empty custom messages.
    pub fn new<F>(init_fn: F) -> Self
    where
        F: FnOnce(
            &mut Router<
                BankKeeper,
                FailingModule<Empty, Empty, Empty>,
                WasmKeeper<Empty, Empty>,
                StakeKeeper,
                DistributionKeeper,
                IbcSimpleModule,
                GovFailingModule,
                StargateFailing,
            >,
            &dyn Api,
            &mut dyn Storage,
        ),
    {
        AppBuilder::new().build(init_fn)
    }
}

/// Creates new default `App` implementation working with customized exec and query messages.
/// Outside the `App` implementation to make type elision better.
pub fn custom_app<ExecC, QueryC, F>(init_fn: F) -> BasicApp<ExecC, QueryC>
where
    ExecC: CustomMsg + DeserializeOwned + 'static,
    QueryC: Debug + CustomQuery + DeserializeOwned + 'static,
    F: FnOnce(
        &mut Router<
            BankKeeper,
            FailingModule<ExecC, QueryC, Empty>,
            WasmKeeper<ExecC, QueryC>,
            StakeKeeper,
            DistributionKeeper,
            IbcSimpleModule,
            GovFailingModule,
            StargateFailing,
        >,
        &dyn Api,
        &mut dyn Storage,
    ),
{
    AppBuilder::new_custom().build(init_fn)
}

impl<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT> Querier
    for App<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
where
    CustomT::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    ApiT: Api,
    StorageT: Storage,
    CustomT: Module,
    StakingT: Staking,
    DistrT: Distribution,
    IbcT: Ibc,
    GovT: Gov,
    StargateT: Stargate,
{
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        self.router
            .querier(&self.api, &self.storage, &self.block)
            .raw_query(bin_request)
    }
}

impl<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
    Executor<CustomT::ExecT>
    for App<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
where
    CustomT::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    ApiT: Api,
    StorageT: Storage,
    CustomT: Module,
    StakingT: Staking,
    DistrT: Distribution,
    IbcT: Ibc,
    GovT: Gov,
    StargateT: Stargate,
{
    fn execute(&mut self, sender: Addr, msg: CosmosMsg<CustomT::ExecT>) -> AnyResult<AppResponse> {
        let mut all = self.execute_multi(sender, vec![msg])?;
        let res = all.pop().unwrap();
        Ok(res)
    }
}

impl<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
    App<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
where
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    ApiT: Api,
    StorageT: Storage,
    CustomT: Module,
    StakingT: Staking,
    DistrT: Distribution,
    IbcT: Ibc,
    GovT: Gov,
    StargateT: Stargate,
{
    /// Returns a shared reference to application's router.
    pub fn router(
        &self,
    ) -> &Router<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT> {
        &self.router
    }

    /// Returns a shared reference to application's API.
    pub fn api(&self) -> &ApiT {
        &self.api
    }

    /// Returns a shared reference to application's storage.
    pub fn storage(&self) -> &StorageT {
        &self.storage
    }

    /// Returns a mutable reference to application's storage.
    pub fn storage_mut(&mut self) -> &mut StorageT {
        &mut self.storage
    }

    /// Initializes modules.
    pub fn init_modules<F, T>(&mut self, init_fn: F) -> T
    where
        F: FnOnce(
            &mut Router<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>,
            &dyn Api,
            &mut dyn Storage,
        ) -> T,
    {
        init_fn(&mut self.router, &self.api, &mut self.storage)
    }

    /// Queries a module.
    pub fn read_module<F, T>(&self, query_fn: F) -> T
    where
        F: FnOnce(
            &Router<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>,
            &dyn Api,
            &dyn Storage,
        ) -> T,
    {
        query_fn(&self.router, &self.api, &self.storage)
    }
}

// Helper functions to call some custom WasmKeeper logic.
// They show how we can easily add such calls to other custom keepers (CustomT, StakingT, etc)
impl<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
    App<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
where
    BankT: Bank,
    ApiT: Api,
    StorageT: Storage,
    CustomT: Module,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    StakingT: Staking,
    DistrT: Distribution,
    IbcT: Ibc,
    GovT: Gov,
    StargateT: Stargate,
    CustomT::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
{
    /// Registers contract code (like uploading wasm bytecode on a chain),
    /// so it can later be used to instantiate a contract.
    pub fn store_code(&mut self, code: Box<dyn Contract<CustomT::ExecT, CustomT::QueryT>>) -> u64 {
        self.router
            .wasm
            .store_code(MockApi::default().addr_make("creator"), code)
    }

    /// Registers contract code (like [store_code](Self::store_code)),
    /// but takes the address of the code creator as an additional argument.
    pub fn store_code_with_creator(
        &mut self,
        creator: Addr,
        code: Box<dyn Contract<CustomT::ExecT, CustomT::QueryT>>,
    ) -> u64 {
        self.router.wasm.store_code(creator, code)
    }

    /// Registers contract code (like [store_code_with_creator](Self::store_code_with_creator)),
    /// but takes the code identifier as an additional argument.
    pub fn store_code_with_id(
        &mut self,
        creator: Addr,
        code_id: u64,
        code: Box<dyn Contract<CustomT::ExecT, CustomT::QueryT>>,
    ) -> AnyResult<u64> {
        self.router.wasm.store_code_with_id(creator, code_id, code)
    }

    /// Duplicates the contract code identified by `code_id` and returns
    /// the identifier of the newly created copy of the contract code.
    ///
    /// # Examples
    ///
    /// ```
    /// use cosmwasm_std::Addr;
    /// use cw_multi_test::App;
    ///
    /// // contract implementation
    /// mod echo {
    ///   // contract entry points not shown here
    /// #  use std::todo;
    /// #  use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError, SubMsg, WasmMsg};
    /// #  use serde::{Deserialize, Serialize};
    /// #  use cw_multi_test::{Contract, ContractWrapper};
    /// #
    /// #  fn instantiate(_: DepsMut, _: Env, _: MessageInfo, _: Empty) -> Result<Response, StdError> {
    /// #    todo!()
    /// #  }
    /// #
    /// #  fn execute(_: DepsMut, _: Env, _info: MessageInfo, msg: WasmMsg) -> Result<Response, StdError> {
    /// #    todo!()
    /// #  }
    /// #
    /// #  fn query(_deps: Deps, _env: Env, _msg: Empty) -> Result<Binary, StdError> {
    /// #    todo!()
    /// #  }
    /// #
    ///   pub fn contract() -> Box<dyn Contract<Empty>> {
    ///     // should return the contract
    /// #   Box::new(ContractWrapper::new(execute, instantiate, query))
    ///   }
    /// }
    ///
    /// let mut app = App::default();
    ///
    /// // store a new contract, save the code id
    /// let code_id = app.store_code(echo::contract());
    ///
    /// // duplicate the existing contract, duplicated contract has different code id
    /// assert_ne!(code_id, app.duplicate_code(code_id).unwrap());
    ///
    /// // zero is an invalid identifier for contract code, returns an error
    /// assert_eq!("code id: invalid", app.duplicate_code(0).unwrap_err().to_string());
    ///
    /// // there is no contract code with identifier 100 stored yet, returns an error
    /// assert_eq!("code id 100: no such code", app.duplicate_code(100).unwrap_err().to_string());
    /// ```
    pub fn duplicate_code(&mut self, code_id: u64) -> AnyResult<u64> {
        self.router.wasm.duplicate_code(code_id)
    }

    /// Returns `ContractData` for the contract with specified address.
    pub fn contract_data(&self, address: &Addr) -> AnyResult<ContractData> {
        self.router.wasm.contract_data(&self.storage, address)
    }

    /// Returns a raw state dump of all key-values held by a contract with specified address.
    pub fn dump_wasm_raw(&self, address: &Addr) -> Vec<Record> {
        self.router.wasm.dump_wasm_raw(&self.storage, address)
    }

    /// Returns **read-only** storage for a contract with specified address.
    pub fn contract_storage<'a>(&'a self, contract_addr: &Addr) -> Box<dyn Storage + 'a> {
        self.router
            .wasm
            .contract_storage(&self.storage, contract_addr)
    }

    /// Returns **read-write** storage for a contract with specified address.
    pub fn contract_storage_mut<'a>(&'a mut self, contract_addr: &Addr) -> Box<dyn Storage + 'a> {
        self.router
            .wasm
            .contract_storage_mut(&mut self.storage, contract_addr)
    }

    /// Returns **read-only** prefixed storage with specified namespace.
    pub fn prefixed_storage<'a>(&'a self, namespace: &[u8]) -> Box<dyn Storage + 'a> {
        Box::new(prefixed_read(&self.storage, namespace))
    }

    /// Returns **mutable** prefixed storage with specified namespace.
    pub fn prefixed_storage_mut<'a>(&'a mut self, namespace: &[u8]) -> Box<dyn Storage + 'a> {
        Box::new(prefixed(&mut self.storage, namespace))
    }

    /// Returns **read-only** prefixed, multilevel storage with specified namespaces.
    pub fn prefixed_multilevel_storage<'a>(
        &'a self,
        namespaces: &[&[u8]],
    ) -> Box<dyn Storage + 'a> {
        Box::new(prefixed_multilevel_read(&self.storage, namespaces))
    }

    /// Returns **mutable** prefixed, multilevel storage with specified namespaces.
    pub fn prefixed_multilevel_storage_mut<'a>(
        &'a mut self,
        namespaces: &[&[u8]],
    ) -> Box<dyn Storage + 'a> {
        Box::new(prefixed_multilevel(&mut self.storage, namespaces))
    }
}

impl<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
    App<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
where
    CustomT::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    ApiT: Api,
    StorageT: Storage,
    CustomT: Module,
    StakingT: Staking,
    DistrT: Distribution,
    IbcT: Ibc,
    GovT: Gov,
    StargateT: Stargate,
{
    /// Sets the initial block properties.
    pub fn set_block(&mut self, block: BlockInfo) {
        self.block = block;
        self.router
            .staking
            .process_queue(&self.api, &mut self.storage, &self.router, &self.block)
            .unwrap();
    }

    /// Updates the current block applying the specified closure, usually [next_block].
    pub fn update_block<F: Fn(&mut BlockInfo)>(&mut self, action: F) {
        action(&mut self.block);
        self.router
            .staking
            .process_queue(&self.api, &mut self.storage, &self.router, &self.block)
            .unwrap();
    }

    /// Returns a copy of the current block info.
    pub fn block_info(&self) -> BlockInfo {
        self.block.clone()
    }

    /// Simple helper so we get access to all the QuerierWrapper helpers,
    /// e.g. wrap().query_wasm_smart, query_all_balances, ...
    pub fn wrap(&self) -> QuerierWrapper<CustomT::QueryT> {
        QuerierWrapper::new(self)
    }

    /// Runs multiple CosmosMsg in one atomic operation.
    /// This will create a cache before the execution, so no state changes are persisted if any of them
    /// return an error. But all writes are persisted on success.
    pub fn execute_multi(
        &mut self,
        sender: Addr,
        msgs: Vec<CosmosMsg<CustomT::ExecT>>,
    ) -> AnyResult<Vec<AppResponse>> {
        // we need to do some caching of storage here, once in the entry point:
        // meaning, wrap current state, all writes go to a cache, only when execute
        // returns a success do we flush it (otherwise drop it)

        let Self {
            block,
            router,
            api,
            storage,
        } = self;

        transactional(&mut *storage, |write_cache, _| {
            msgs.into_iter()
                .map(|msg| router.execute(&*api, write_cache, block, sender.clone(), msg))
                .collect()
        })
    }

    /// Call a smart contract in "sudo" mode.
    /// This will create a cache before the execution, so no state changes are persisted if this
    /// returns an error, but all are persisted on success.
    pub fn wasm_sudo<T: Serialize, U: Into<Addr>>(
        &mut self,
        contract_addr: U,
        msg: &T,
    ) -> AnyResult<AppResponse> {
        let msg = WasmSudo {
            contract_addr: contract_addr.into(),
            message: to_json_binary(msg)?,
        };

        let Self {
            block,
            router,
            api,
            storage,
        } = self;

        transactional(&mut *storage, |write_cache, _| {
            router.wasm.sudo(&*api, write_cache, router, block, msg)
        })
    }

    /// Queries the IBC module
    pub fn ibc_query(&self, query: MockIbcQuery) -> AnyResult<Binary> {
        let Self {
            block,
            router,
            api,
            storage,
        } = self;

        let querier = router.querier(api, storage, block);

        router.ibc.query(api, storage, &querier, block, query)
    }

    /// Runs arbitrary SudoMsg.
    /// This will create a cache before the execution, so no state changes are persisted if this
    /// returns an error, but all are persisted on success.
    pub fn sudo(&mut self, msg: SudoMsg) -> AnyResult<AppResponse> {
        // we need to do some caching of storage here, once in the entry point:
        // meaning, wrap current state, all writes go to a cache, only when execute
        // returns a success do we flush it (otherwise drop it)
        let Self {
            block,
            router,
            api,
            storage,
        } = self;

        transactional(&mut *storage, |write_cache, _| {
            router.sudo(&*api, write_cache, block, msg)
        })
    }
}
/// The Router plays a critical role in managing and directing
/// transactions within the Cosmos blockchain.
#[derive(Clone)]
pub struct Router<Bank, Custom, Wasm, Staking, Distr, Ibc, Gov, Stargate> {
    /// Wasm module instance to be used in this [Router].
    pub(crate) wasm: Wasm,
    /// Bank module instance to be used in this [Router].
    pub bank: Bank,
    /// Custom module instance to be used in this [Router].
    pub custom: Custom,
    /// Staking module instance to be used in this [Router].
    pub staking: Staking,
    /// Distribution module instance to be used in this [Router].
    pub distribution: Distr,
    /// IBC module instance to be used in this [Router].
    pub ibc: Ibc,
    /// Governance module instance to be used in this [Router].
    pub gov: Gov,
    /// Stargate handler instance to be used in this [Router].
    pub stargate: Stargate,
}

impl<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
    Router<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
where
    CustomT::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
    CustomT: Module,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    StakingT: Staking,
    DistrT: Distribution,
    IbcT: Ibc,
    GovT: Gov,
    StargateT: Stargate,
{
    /// Returns a querier populated with the instance of this [Router].
    pub fn querier<'a>(
        &'a self,
        api: &'a dyn Api,
        storage: &'a dyn Storage,
        block_info: &'a BlockInfo,
    ) -> RouterQuerier<'a, CustomT::ExecT, CustomT::QueryT> {
        RouterQuerier {
            router: self,
            api,
            storage,
            block_info,
        }
    }
}

/// We use it to allow calling into modules from another module in sudo mode.
/// Things like gov proposals belong here.
pub enum SudoMsg {
    /// Bank privileged actions.
    Bank(BankSudo),
    /// Custom privileged actions.
    Custom(Empty),
    /// Staking privileged actions.
    Staking(StakingSudo),
    /// Wasm privileged actions.
    Wasm(WasmSudo),
    /// Ibc actions, used namely to create channels and relay packets
    Ibc(IbcSudo),
}

impl From<WasmSudo> for SudoMsg {
    fn from(wasm: WasmSudo) -> Self {
        SudoMsg::Wasm(wasm)
    }
}

impl From<BankSudo> for SudoMsg {
    fn from(bank: BankSudo) -> Self {
        SudoMsg::Bank(bank)
    }
}

impl From<StakingSudo> for SudoMsg {
    fn from(staking: StakingSudo) -> Self {
        SudoMsg::Staking(staking)
    }
}

/// We use it to allow calling into modules from the ibc module. This is used for receiving packets
pub struct IbcRouterMsg {
    pub module: IbcModule,
    pub msg: IbcModuleMsg,
}

#[cosmwasm_schema::cw_serde]
pub enum IbcModule {
    Wasm(Addr), // The wasm module needs to contain the wasm contract address (usually decoded from the port)
    Bank,
    Staking,
}

/// A trait representing the Cosmos based chain's router.
///
/// This trait is designed for routing messages within the Cosmos ecosystem.
/// It is key to ensure that transactions and contract calls are directed to the
/// correct destinations during testing, simulating real-world blockchain operations.
pub trait CosmosRouter {
    /// Type of the executed custom message.
    type ExecC: CustomMsg;
    /// Type of the query custom message.
    type QueryC: CustomQuery;

    /// Executes messages.
    fn execute(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        sender: Addr,
        msg: CosmosMsg<Self::ExecC>,
    ) -> AnyResult<AppResponse>;

    /// Evaluates queries.
    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        block: &BlockInfo,
        request: QueryRequest<Self::QueryC>,
    ) -> AnyResult<Binary>;

    /// Evaluates privileged actions.
    fn sudo(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        msg: SudoMsg,
    ) -> AnyResult<AppResponse>;

    /// Evaluates all ibc related actions
    fn ibc(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        msg: IbcRouterMsg,
    ) -> AnyResult<IbcResponse>;
}

impl<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT> CosmosRouter
    for Router<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
where
    CustomT::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
    CustomT: Module,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    StakingT: Staking,
    DistrT: Distribution,
    IbcT: Ibc,
    GovT: Gov,
    StargateT: Stargate,
{
    type ExecC = CustomT::ExecT;
    type QueryC = CustomT::QueryT;

    fn execute(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        sender: Addr,
        msg: CosmosMsg<Self::ExecC>,
    ) -> AnyResult<AppResponse> {
        match msg {
            CosmosMsg::Wasm(msg) => self.wasm.execute(api, storage, self, block, sender, msg),
            CosmosMsg::Bank(msg) => self.bank.execute(api, storage, self, block, sender, msg),
            CosmosMsg::Custom(msg) => self.custom.execute(api, storage, self, block, sender, msg),
            CosmosMsg::Staking(msg) => self.staking.execute(api, storage, self, block, sender, msg),
            CosmosMsg::Distribution(msg) => self
                .distribution
                .execute(api, storage, self, block, sender, msg),
            CosmosMsg::Ibc(msg) => self.ibc.execute(api, storage, self, block, sender, msg),
            CosmosMsg::Gov(msg) => self.gov.execute(api, storage, self, block, sender, msg),
            #[allow(deprecated)]
            CosmosMsg::Stargate { type_url, value } => self
                .stargate
                .execute_stargate(api, storage, self, block, sender, type_url, value),
            #[cfg(feature = "cosmwasm_2_0")]
            CosmosMsg::Any(msg) => self
                .stargate
                .execute_any(api, storage, self, block, sender, msg),
            _ => bail!("Cannot execute {:?}", msg),
        }
    }

    /// This is used by `RouterQuerier` to actual implement the `Querier` interface.
    /// you most likely want to use `router.querier(storage, block).wrap()` to get a
    /// QuerierWrapper to interact with
    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        block: &BlockInfo,
        request: QueryRequest<Self::QueryC>,
    ) -> AnyResult<Binary> {
        let querier = self.querier(api, storage, block);
        match request {
            QueryRequest::Wasm(req) => self.wasm.query(api, storage, &querier, block, req),
            QueryRequest::Bank(req) => self.bank.query(api, storage, &querier, block, req),
            QueryRequest::Custom(req) => self.custom.query(api, storage, &querier, block, req),
            QueryRequest::Staking(req) => self.staking.query(api, storage, &querier, block, req),
            QueryRequest::Ibc(req) => self.ibc.query(api, storage, &querier, block, req.into()),
            #[allow(deprecated)]
            QueryRequest::Stargate { path, data } => self
                .stargate
                .query_stargate(api, storage, &querier, block, path, data),
            #[cfg(feature = "cosmwasm_2_0")]
            QueryRequest::Grpc(req) => self.stargate.query_grpc(api, storage, &querier, block, req),
            _ => unimplemented!(),
        }
    }

    fn sudo(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        msg: SudoMsg,
    ) -> AnyResult<AppResponse> {
        match msg {
            SudoMsg::Wasm(msg) => self.wasm.sudo(api, storage, self, block, msg),
            SudoMsg::Bank(msg) => self.bank.sudo(api, storage, self, block, msg),
            SudoMsg::Staking(msg) => self.staking.sudo(api, storage, self, block, msg),
            SudoMsg::Custom(_) => unimplemented!(),
            SudoMsg::Ibc(msg) => self.ibc.sudo(api, storage, self, block, msg),
        }
    }

    fn ibc(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        msg: IbcRouterMsg,
    ) -> AnyResult<IbcResponse> {
        match msg.module {
            IbcModule::Bank => match msg.msg {
                IbcModuleMsg::ChannelOpen(m) => self
                    .bank
                    .ibc_channel_open(api, storage, self, block, m)
                    .map(Into::into),
                IbcModuleMsg::ChannelConnect(m) => self
                    .bank
                    .ibc_channel_connect(api, storage, self, block, m)
                    .map(Into::into),
                IbcModuleMsg::ChannelClose(m) => self
                    .bank
                    .ibc_channel_close(api, storage, self, block, m)
                    .map(Into::into),
                IbcModuleMsg::PacketReceive(m) => self
                    .bank
                    .ibc_packet_receive(api, storage, self, block, m)
                    .map(Into::into),
                IbcModuleMsg::PacketAcknowledgement(m) => self
                    .bank
                    .ibc_packet_acknowledge(api, storage, self, block, m)
                    .map(Into::into),
                IbcModuleMsg::PacketTimeout(m) => self
                    .bank
                    .ibc_packet_timeout(api, storage, self, block, m)
                    .map(Into::into),
            },
            IbcModule::Staking => match msg.msg {
                IbcModuleMsg::ChannelOpen(m) => self
                    .staking
                    .ibc_channel_open(api, storage, self, block, m)
                    .map(Into::into),
                IbcModuleMsg::ChannelConnect(m) => self
                    .staking
                    .ibc_channel_connect(api, storage, self, block, m)
                    .map(Into::into),
                IbcModuleMsg::ChannelClose(m) => self
                    .staking
                    .ibc_channel_close(api, storage, self, block, m)
                    .map(Into::into),
                IbcModuleMsg::PacketReceive(m) => self
                    .staking
                    .ibc_packet_receive(api, storage, self, block, m)
                    .map(Into::into),
                IbcModuleMsg::PacketAcknowledgement(m) => self
                    .staking
                    .ibc_packet_acknowledge(api, storage, self, block, m)
                    .map(Into::into),
                IbcModuleMsg::PacketTimeout(m) => self
                    .staking
                    .ibc_packet_timeout(api, storage, self, block, m)
                    .map(Into::into),
            },
            IbcModule::Wasm(contract_addr) => match msg.msg {
                IbcModuleMsg::ChannelOpen(m) => self
                    .wasm
                    .ibc_channel_open(api, contract_addr, storage, self, block, m)
                    .map(Into::into),
                IbcModuleMsg::ChannelConnect(m) => self
                    .wasm
                    .ibc_channel_connect(api, contract_addr, storage, self, block, m)
                    .map(Into::into),
                IbcModuleMsg::ChannelClose(m) => self
                    .wasm
                    .ibc_channel_close(api, contract_addr, storage, self, block, m)
                    .map(Into::into),
                IbcModuleMsg::PacketReceive(m) => self
                    .wasm
                    .ibc_packet_receive(api, contract_addr, storage, self, block, m)
                    .map(Into::into),
                IbcModuleMsg::PacketAcknowledgement(m) => self
                    .wasm
                    .ibc_packet_acknowledge(api, contract_addr, storage, self, block, m)
                    .map(Into::into),
                IbcModuleMsg::PacketTimeout(m) => self
                    .wasm
                    .ibc_packet_timeout(api, contract_addr, storage, self, block, m)
                    .map(Into::into),
            },
        }
    }
}

pub struct MockRouter<ExecC, QueryC>(PhantomData<(ExecC, QueryC)>);

impl Default for MockRouter<Empty, Empty> {
    fn default() -> Self {
        Self::new()
    }
}

impl<ExecC, QueryC> MockRouter<ExecC, QueryC> {
    pub fn new() -> Self
    where
        QueryC: CustomQuery,
    {
        MockRouter(PhantomData)
    }
}

impl<ExecC, QueryC> CosmosRouter for MockRouter<ExecC, QueryC>
where
    ExecC: CustomMsg,
    QueryC: CustomQuery,
{
    type ExecC = ExecC;
    type QueryC = QueryC;

    fn execute(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _block: &BlockInfo,
        _sender: Addr,
        _msg: CosmosMsg<Self::ExecC>,
    ) -> AnyResult<AppResponse> {
        panic!("Cannot execute MockRouters");
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _block: &BlockInfo,
        _request: QueryRequest<Self::QueryC>,
    ) -> AnyResult<Binary> {
        panic!("Cannot query MockRouters");
    }

    fn sudo(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _block: &BlockInfo,
        _msg: SudoMsg,
    ) -> AnyResult<AppResponse> {
        panic!("Cannot sudo MockRouters");
    }

    fn ibc(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _block: &BlockInfo,
        _msg: IbcRouterMsg,
    ) -> AnyResult<IbcResponse> {
        panic!("Cannot ibc MockRouters");
    }
}

pub struct RouterQuerier<'a, ExecC, QueryC> {
    router: &'a dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
    api: &'a dyn Api,
    storage: &'a dyn Storage,
    block_info: &'a BlockInfo,
}

impl<'a, ExecC, QueryC> RouterQuerier<'a, ExecC, QueryC> {
    pub fn new(
        router: &'a dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        api: &'a dyn Api,
        storage: &'a dyn Storage,
        block_info: &'a BlockInfo,
    ) -> Self {
        Self {
            router,
            api,
            storage,
            block_info,
        }
    }
}

impl<'a, ExecC, QueryC> Querier for RouterQuerier<'a, ExecC, QueryC>
where
    ExecC: CustomMsg + DeserializeOwned + 'static,
    QueryC: CustomQuery + DeserializeOwned + 'static,
{
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<QueryC> = match from_json(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };
        let contract_result: ContractResult<Binary> = self
            .router
            .query(self.api, self.storage, self.block_info, request)
            .into();
        SystemResult::Ok(contract_result)
    }
}
