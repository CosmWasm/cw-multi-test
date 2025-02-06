//! # Implementation of the contract trait and the contract wrapper

use crate::error::{anyhow, bail, AnyError, AnyResult};
use cosmwasm_std::{
    from_json, Binary, Checksum, CosmosMsg, CustomMsg, CustomQuery, Deps, DepsMut, Empty, Env,
    IbcDestinationCallbackMsg, IbcSourceCallbackMsg, MessageInfo, QuerierWrapper, Reply, Response,
    SubMsg,
};
use cosmwasm_std::{
    IbcBasicResponse, IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg,
    IbcChannelOpenResponse, IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg,
    IbcReceiveResponse,
};
use serde::de::DeserializeOwned;
use std::fmt::{Debug, Display};
use std::ops::Deref;

/// A primary interface for interacting with smart contracts.
#[rustfmt::skip]
pub trait Contract<C, Q = Empty>
where
    C: CustomMsg,
    Q: CustomQuery,
{
    /// Evaluates contract's `execute` entry-point.
    fn execute(&self, deps: DepsMut<Q>, env: Env, info: MessageInfo, msg: Vec<u8>) -> AnyResult<Response<C>>;

    /// Evaluates contract's `instantiate` entry-point.
    fn instantiate(&self, deps: DepsMut<Q>, env: Env, info: MessageInfo, msg: Vec<u8>) -> AnyResult<Response<C>>;

    /// Evaluates contract's `query` entry-point.
    fn query(&self, deps: Deps<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Binary>;

    /// Evaluates contract's `sudo` entry-point.
    fn sudo(&self, deps: DepsMut<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Response<C>>;

    /// Evaluates contract's `reply` entry-point.
    fn reply(&self, deps: DepsMut<Q>, env: Env, msg: Reply) -> AnyResult<Response<C>>;

    /// Evaluates contract's `migrate` entry-point.
    fn migrate(&self, deps: DepsMut<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Response<C>>;

    /// Evaluates the contract's `ibc_channel_open` entry-point.
    fn ibc_channel_open(&self, deps: DepsMut<Q>, env: Env, msg: IbcChannelOpenMsg) -> AnyResult<IbcChannelOpenResponse>;

    /// Evaluates the contract's `ibc_channel_connect` entry-point.
    fn ibc_channel_connect(&self, deps: DepsMut<Q>, env: Env, msg: IbcChannelConnectMsg) -> AnyResult<IbcBasicResponse<C>>;

    /// Evaluates the contract's `ibc_channel_close` entry-point.
    fn ibc_channel_close(&self, deps: DepsMut<Q>, env: Env, msg: IbcChannelCloseMsg) -> AnyResult<IbcBasicResponse<C>>;

    /// Evaluates the contract's `ibc_packet_receive` entry-point.
    fn ibc_packet_receive(&self, deps: DepsMut<Q>, env: Env, msg: IbcPacketReceiveMsg) -> AnyResult<IbcReceiveResponse<C>>;

    /// Evaluates the contract's `ibc_packet_ack` entry-point.
    fn ibc_packet_ack(&self, deps: DepsMut<Q>, env: Env, msg: IbcPacketAckMsg) -> AnyResult<IbcBasicResponse<C>>;

    /// Evaluates the contract's `ibc_packet_timeout` entry-point.
    fn ibc_packet_timeout(&self, deps: DepsMut<Q>, env: Env, msg: IbcPacketTimeoutMsg) -> AnyResult<IbcBasicResponse<C>>;

    /// Evaluates the contract's `ibc_source_callback` entry-point.
    fn ibc_source_callback(&self, deps: DepsMut<Q>, env: Env, msg: IbcSourceCallbackMsg) -> AnyResult<IbcBasicResponse<C>>;

    /// Evaluates the contract's `ibc_destination_callback` entry-point.
    fn ibc_destination_callback(&self, deps: DepsMut<Q>, env: Env, msg: IbcDestinationCallbackMsg) -> AnyResult<IbcBasicResponse<C>>;

    /// Returns the provided checksum of the contract's Wasm blob.
    fn checksum(&self) -> Option<Checksum> {
        None
    }
}

#[rustfmt::skip]
mod closures {
    use super::*;

    // Function types:
    pub type ContractFn<T, C, E, Q> = fn(deps: DepsMut<Q>, env: Env, info: MessageInfo, msg: T) -> Result<Response<C>, E>;
    pub type PermissionedFn<T, C, E, Q> = fn(deps: DepsMut<Q>, env: Env, msg: T) -> Result<Response<C>, E>;
    pub type ReplyFn<C, E, Q> = fn(deps: DepsMut<Q>, env: Env, msg: Reply) -> Result<Response<C>, E>;
    pub type QueryFn<T, E, Q> = fn(deps: Deps<Q>, env: Env, msg: T) -> Result<Binary, E>;
    pub type IbcFn<T, R, E, Q> = fn(deps: DepsMut<Q>, env: Env, msg: T) -> Result<R, E>;

    // Closure types:
    pub type ContractClosure<T, C, E, Q> = Box<dyn Fn(DepsMut<Q>, Env, MessageInfo, T) -> Result<Response<C>, E>>;
    pub type PermissionedClosure<T, C, E, Q> = Box<dyn Fn(DepsMut<Q>, Env, T) -> Result<Response<C>, E>>;
    pub type ReplyClosure<C, E, Q> = Box<dyn Fn(DepsMut<Q>, Env, Reply) -> Result<Response<C>, E>>;
    pub type QueryClosure<T, E, Q> = Box<dyn Fn(Deps<Q>, Env, T) -> Result<Binary, E>>;
    pub type IbcClosure<T, R, E, Q> = Box<dyn Fn(DepsMut<Q>,Env, T) -> Result<R, E>>;
}

use closures::*;

/// This structure wraps the [Contract] trait implementor
/// and provides generic access to the contract's entry-points.
///
/// List of generic types used in [ContractWrapper]:
/// - **T1** type of message passed to [execute] entry-point.
/// - **T2** type of message passed to [instantiate] entry-point.
/// - **T3** type of message passed to [query] entry-point.
/// - **T4** type of message passed to [sudo] entry-point.
/// - instead of **~~T5~~**, always the `Reply` type is used in [reply] entry-point.
/// - **T6** type of message passed to [migrate] entry-point.
/// - **E1** type of error returned from [execute] entry-point.
/// - **E2** type of error returned from [instantiate] entry-point.
/// - **E3** type of error returned from [query] entry-point.
/// - **E4** type of error returned from [sudo] entry-point.
/// - **E5** type of error returned from [reply] entry-point.
/// - **E6** type of error returned from [migrate] entry-point.
/// - **E7** type of error returned from [ibc_channel_open] entry-point.
/// - **E8** type of error returned from [ibc_channel_connect] entry-point.
/// - **E9** type of error returned from [ibc_channel_close] entry-point.
/// - **E10** type of error returned from [ibc_packet_receive] entry-point.
/// - **E11** type of error returned from [ibc_packet_ack] entry-point.
/// - **E12** type of error returned from [ibc_packet_timeout] entry-point.
/// - **E13** type of error returned from [ibc_source_callback] entry-point.
/// - **E14** type of error returned from [ibc_destination_callback] entry-point.
/// - **C** type of custom message returned from all entry-points except [query].
/// - **Q** type of custom query in `Querier` passed as 'Deps' or 'DepsMut' to all entry-points.
///
/// The following table summarizes the purpose of all generic types used in [ContractWrapper].
/// ```text
/// ┌──────────────────────────┬─────────────────────────────┬─────────────────────┬───────────────────────────┬────────────────────────┬───────┬───────┐
/// │  Contract entry-point    │  ContractWrapper function   │    Closure type     │        message_in         │      message_out       │       │       │
/// │                          │                             │                     │                           │                        │ Error │ Query │
/// │                          │                             │                     │                           │                        │  OUT  │       │
/// ╞══════════════════════════╪═════════════════════════════╪═════════════════════╪═══════════════════════════╪════════════════════════╪═══════╪═══════╡
/// │ execute                  │ execute_fn                  │ ContractClosure     │ T1                        │ C                      │  E1   │   Q   │
/// ├──────────────────────────┼─────────────────────────────┼─────────────────────┼───────────────────────────┼────────────────────────┼───────┼───────┤
/// │ instantiate              │ instantiate_fn              │ ContractClosure     │ T2                        │ C                      │  E2   │   Q   │
/// ├──────────────────────────┼─────────────────────────────┼─────────────────────┼───────────────────────────┼────────────────────────┼───────┼───────┤
/// │ query                    │ query_fn                    │ QueryClosure        │ T3                        │ Binary                 │  E3   │   Q   │
/// ├──────────────────────────┼─────────────────────────────┼─────────────────────┼───────────────────────────┼────────────────────────┼───────┼───────┤
/// │ sudo                     │ sudo_fn                     │ PermissionedClosure │ T4                        │ C                      │  E4   │   Q   │
/// ├──────────────────────────┼─────────────────────────────┼─────────────────────┼───────────────────────────┼────────────────────────┼───────┼───────┤
/// │ reply                    │ reply_fn                    │ ReplyClosure        │ Reply                     │ C                      │  E5   │   Q   │
/// ├──────────────────────────┼─────────────────────────────┼─────────────────────┼───────────────────────────┼────────────────────────┼───────┼───────┤
/// │ migrate                  │ migrate_fn                  │ PermissionedClosure │ T6                        │ C                      │  E6   │   Q   │
/// ├──────────────────────────┼─────────────────────────────┼─────────────────────┼───────────────────────────┼────────────────────────┼───────┼───────┤
/// │ ibc_channel_open         │ ibc_channel_open_fn         │ IbcClosure          │ IbcChannelOpenMsg         │ IbcChannelOpenResponse │  E7   │   Q   │
/// ├──────────────────────────┼─────────────────────────────┼─────────────────────┼───────────────────────────┼────────────────────────┼───────┼───────┤
/// │ ibc_channel_connect      │ ibc_channel_connect_fn      │ IbcClosure          │ IbcChannelConnectMsg      │ IbcBasicResponse       │  E8   │   Q   │
/// ├──────────────────────────┼─────────────────────────────┼─────────────────────┼───────────────────────────┼────────────────────────┼───────┼───────┤
/// │ ibc_channel_close        │ ibc_channel_close_fn        │ IbcClosure          │ IbcChannelCloseMsg        │ IbcBasicResponse       │  E9   │   Q   │
/// ├──────────────────────────┼─────────────────────────────┼─────────────────────┼───────────────────────────┼────────────────────────┼───────┼───────┤
/// │ ibc_packet_receive       │ ibc_packet_receive_fn       │ IbcClosure          │ IbcPacketReceiveMsg       │ IbcReceiveResponse     │  E10  │   Q   │
/// ├──────────────────────────┼─────────────────────────────┼─────────────────────┼───────────────────────────┼────────────────────────┼───────┼───────┤
/// │ ibc_packet_ack           │ ibc_packet_ack_fn           │ IbcClosure          │ IbcPacketAckMsg           │ IbcBasicResponse       │  E11  │   Q   │
/// ├──────────────────────────┼─────────────────────────────┼─────────────────────┼───────────────────────────┼────────────────────────┼───────┼───────┤
/// │ ibc_packet_timeout       │ ibc_packet_timeout_fn       │ IbcClosure          │ IbcPacketTimeoutMsg       │ IbcBasicResponse       │  E12  │   Q   │
/// ├──────────────────────────┼─────────────────────────────┼─────────────────────┼───────────────────────────┼────────────────────────┼───────┼───────┤
/// │ ibc_source_callback      │ ibc_source_callback_fn      │ IbcClosure          │ IbcSourceCallbackMsg      │ IbcBasicResponse       │  E13  │   Q   │
/// ├──────────────────────────┼─────────────────────────────┼─────────────────────┼───────────────────────────┼────────────────────────┼───────┼───────┤
/// │ ibc_destination_callback │ ibc_destination_callback_fn │ IbcClosure          │ IbcDestinationCallbackMsg │ IbcBasicResponse       │  E14  │   Q   │
/// └──────────────────────────┴─────────────────────────────┴─────────────────────┴───────────────────────────┴────────────────────────┴───────┴───────┘
/// ```
/// The general schema depicting which generic type is used in entry points is shown below.
/// Entry point, when called, is provided minimum two arguments: custom query of type **Q**
/// (inside `Deps` or `DepsMut`) and input message of type **T1**, **T2**, **T3**, **T4**,
/// **Reply**, **T6**, **IbcChannelOpenMsg**, **IbcChannelConnectMsg**, **IbcChannelCloseMsg**,
/// **IbcPacketReceiveMsg**, **IbcPacketAckMsg**, **IbcPacketTimeoutMsg**, **IbcSourceCallbackMsg**
/// or **IbcDestinationCallbackMsg**. As a result, entry point returns custom output message of type
/// Response<**C**>, **Binary**, **IbcChannelOpenResponse**, **IbcReceiveResponse** or **IbcBasicResponse**
/// and an error of type **E1**, **E2**, **E3**, **E4**, **E5**, **E6**, **E7**, **E8**, **E9**, **E10**,
/// **E11**, **E12**, **E13** or **E14**.
///
/// ```text
///    entry_point(query, .., message_in) -> Result<message_out, error>
///                  ┬           ┬                      ┬          ┬
///             Q >──┘           │                      │          └──> E1,E2,E3,E4,E5,E6,E7,
///                              │                      │               E8,E9,E10,E11,E12,E13,E14
///    T1,T2,T3,T4,Reply,T6,  >──┘                      └──> C,Binary,
///    IbcChannelOpenMsg,                                    IbcChannelOpenResponse,
///    IbcChannelConnectMsg,                                 IbcReceiveResponse,
///    IbcChannelCloseMsg,                                   IbcBasicResponse
///    IbcPacketReceiveMsg,
///    IbcPacketAckMsg,
///    IbcPacketTimeoutMsg,
///    IbcSourceCallbackMsg,
///    IbcDestinationCallbackMsg
/// ```
/// Generic type **C** defines a custom message that is specific for the **whole blockchain**.
/// Similarly, the generic type **Q** defines a custom query that is also specific
/// to the **whole blockchain**. Other generic types are specific to the implemented contract.
/// So all smart contracts used in the same blockchain will have the same types for **C** and **Q**,
/// but each contract may use different type for other generic types.
/// It means that e.g. **T1** in smart contract `A` may differ from **T1** in smart contract `B`.
///
/// [execute]: Contract::execute
/// [instantiate]: Contract::instantiate
/// [query]: Contract::query
/// [sudo]: Contract::sudo
/// [reply]: Contract::reply
/// [migrate]: Contract::migrate
/// [ibc_channel_open]: Contract::ibc_channel_open
/// [ibc_channel_connect]: Contract::ibc_channel_connect
/// [ibc_channel_close]: Contract::ibc_channel_close
/// [ibc_packet_receive]: Contract::ibc_packet_receive
/// [ibc_packet_ack]: Contract::ibc_packet_ack
/// [ibc_packet_timeout]: Contract::ibc_packet_timeout
/// [ibc_source_callback]: Contract::ibc_source_callback
/// [ibc_destination_callback]: Contract::ibc_destination_callback
#[rustfmt::skip]
pub struct ContractWrapper<
    T1,
    T2,
    T3,
    E1,
    E2,
    E3,
    C = Empty,
    Q = Empty,
    T4 = Empty,
    E4 = AnyError,
    E5 = AnyError,
    T6 = Empty,
    E6 = AnyError,
    E7 = AnyError,
    E8 = AnyError,
    E9 = AnyError,
    E10 = AnyError,
    E11 = AnyError,
    E12 = AnyError,
    E13 = AnyError,
    E14 = AnyError,
> where
    T1: DeserializeOwned,               // Type of message passed to `execute` entry-point.
    T2: DeserializeOwned,               // Type of message passed to `instantiate` entry-point.
    T3: DeserializeOwned,               // Type of message passed to `query` entry-point.
    T4: DeserializeOwned,               // Type of message passed to `sudo` entry-point.
    T6: DeserializeOwned,               // Type of message passed to `migrate` entry-point.
    E1: Display + Debug + Send + Sync,  // Type of error returned from `execute` entry-point.
    E2: Display + Debug + Send + Sync,  // Type of error returned from `instantiate` entry-point.
    E3: Display + Debug + Send + Sync,  // Type of error returned from `query` entry-point.
    E4: Display + Debug + Send + Sync,  // Type of error returned from `sudo` entry-point.
    E5: Display + Debug + Send + Sync,  // Type of error returned from `reply` entry-point.
    E6: Display + Debug + Send + Sync,  // Type of error returned from `migrate` entry-point.
    E7: Display + Debug + Send + Sync,  // Type of error returned from `ibc_channel_open` entry-point.
    E8: Display + Debug + Send + Sync,  // Type of error returned from `ibc_channel_connect` entry-point.
    E9: Display + Debug + Send + Sync,  // Type of error returned from `ibc_channel_close` entry-point.
    E10: Display + Debug + Send + Sync, // Type of error returned from `ibc_packet_receive` entry-point.
    E11: Display + Debug + Send + Sync, // Type of error returned from `ibc_packet_ack` entry-point.
    E12: Display + Debug + Send + Sync, // Type of error returned from `ibc_packet_timeout` entry-point.
    E13: Display + Debug + Send + Sync, // Type of error returned from `ibc_source_callback_fn` entry-point.
    E14: Display + Debug + Send + Sync, // Type of error returned from `ibc_destination_callback_fn` entry-point.
    C: CustomMsg,                       // Type of custom message returned from all entry-points except `query`.
    Q: CustomQuery + DeserializeOwned,  // Type of custom query in querier passed as deps/deps_mut to all entry-points.
{
    execute_fn: ContractClosure<T1, C, E1, Q>,
    instantiate_fn: ContractClosure<T2, C, E2, Q>,
    query_fn: QueryClosure<T3, E3, Q>,
    sudo_fn: Option<PermissionedClosure<T4, C, E4, Q>>,
    reply_fn: Option<ReplyClosure<C, E5, Q>>,
    migrate_fn: Option<PermissionedClosure<T6, C, E6, Q>>,
    ibc_channel_open_fn: Option<IbcClosure<IbcChannelOpenMsg, IbcChannelOpenResponse, E7, Q>>,
    ibc_channel_connect_fn: Option<IbcClosure<IbcChannelConnectMsg, IbcBasicResponse<C>, E8, Q>>,
    ibc_channel_close_fn: Option<IbcClosure<IbcChannelCloseMsg, IbcBasicResponse<C>, E9, Q>>,
    ibc_packet_receive_fn: Option<IbcClosure<IbcPacketReceiveMsg, IbcReceiveResponse<C>, E10, Q>>,
    ibc_packet_ack_fn: Option<IbcClosure<IbcPacketAckMsg, IbcBasicResponse<C>, E11, Q>>,
    ibc_packet_timeout_fn: Option<IbcClosure<IbcPacketTimeoutMsg, IbcBasicResponse<C>, E12, Q>>,
    ibc_source_callback_fn: Option<IbcClosure<IbcSourceCallbackMsg, IbcBasicResponse<C>, E13, Q>>,
    ibc_destination_callback_fn: Option<IbcClosure<IbcDestinationCallbackMsg, IbcBasicResponse<C>, E14, Q>>,
    checksum: Option<Checksum>,
}

impl<T1, T2, T3, E1, E2, E3, C, Q> ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q>
where
    T1: DeserializeOwned + 'static, // Type of message passed to `execute` entry-point.
    T2: DeserializeOwned + 'static, // Type of message passed to `instantiate` entry-point.
    T3: DeserializeOwned + 'static, // Type of message passed to `query` entry-point.
    E1: Display + Debug + Send + Sync + 'static, // Type of error returned from `execute` entry-point.
    E2: Display + Debug + Send + Sync + 'static, // Type of error returned from `instantiate` entry-point.
    E3: Display + Debug + Send + Sync + 'static, // Type of error returned from `query` entry-point.
    C: CustomMsg + 'static, // Type of custom message returned from all entry-points except `query`.
    Q: CustomQuery + DeserializeOwned + 'static, // Type of custom query in querier passed as deps/deps_mut to all entry-points.
{
    /// Creates a new contract wrapper with default settings.
    pub fn new(
        execute_fn: ContractFn<T1, C, E1, Q>,
        instantiate_fn: ContractFn<T2, C, E2, Q>,
        query_fn: QueryFn<T3, E3, Q>,
    ) -> Self {
        Self {
            execute_fn: Box::new(execute_fn),
            instantiate_fn: Box::new(instantiate_fn),
            query_fn: Box::new(query_fn),
            sudo_fn: None,
            reply_fn: None,
            migrate_fn: None,
            ibc_channel_open_fn: None,
            ibc_channel_connect_fn: None,
            ibc_channel_close_fn: None,
            ibc_packet_receive_fn: None,
            ibc_packet_ack_fn: None,
            ibc_packet_timeout_fn: None,
            ibc_source_callback_fn: None,
            ibc_destination_callback_fn: None,
            checksum: None,
        }
    }

    /// This will take a contract that returns `Response<Empty>` and will _upgrade_ it
    /// to `Response<C>` if needed, to be compatible with a chain-specific extension.
    pub fn new_with_empty(
        execute_fn: ContractFn<T1, Empty, E1, Empty>,
        instantiate_fn: ContractFn<T2, Empty, E2, Empty>,
        query_fn: QueryFn<T3, E3, Empty>,
    ) -> Self {
        Self {
            execute_fn: customize_contract_fn(execute_fn),
            instantiate_fn: customize_contract_fn(instantiate_fn),
            query_fn: customize_query_fn(query_fn),
            sudo_fn: None,
            reply_fn: None,
            migrate_fn: None,
            ibc_channel_open_fn: None,
            ibc_channel_connect_fn: None,
            ibc_channel_close_fn: None,
            ibc_packet_receive_fn: None,
            ibc_packet_ack_fn: None,
            ibc_packet_timeout_fn: None,
            ibc_source_callback_fn: None,
            ibc_destination_callback_fn: None,
            checksum: None,
        }
    }
}

#[allow(clippy::type_complexity)]
#[rustfmt::skip]
impl<T1, T2, T3, E1, E2, E3, C, Q, T4, E4, E5, T6, E6>
    ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4, E4, E5, T6, E6>
where
    T1: DeserializeOwned,                        // Type of message passed to `execute` entry-point.
    T2: DeserializeOwned,                        // Type of message passed to `instantiate` entry-point.
    T3: DeserializeOwned,                        // Type of message passed to `query` entry-point.
    T4: DeserializeOwned,                        // Type of message passed to `sudo` entry-point.
    T6: DeserializeOwned,                        // Type of message passed to `migrate` entry-point.
    E1: Display + Debug + Send + Sync,           // Type of error returned from `execute` entry-point.
    E2: Display + Debug + Send + Sync,           // Type of error returned from `instantiate` entry-point.
    E3: Display + Debug + Send + Sync,           // Type of error returned from `query` entry-point.
    E4: Display + Debug + Send + Sync,           // Type of error returned from `sudo` entry-point.
    E5: Display + Debug + Send + Sync,           // Type of error returned from `reply` entry-point.
    E6: Display + Debug + Send + Sync,           // Type of error returned from `migrate` entry-point.
    C: CustomMsg + 'static,                      // Type of custom message returned from all entry-points except `query`.
    Q: CustomQuery + DeserializeOwned + 'static, // Type of custom query in querier passed as deps/deps_mut to all entry-points.
{
    /// Populates [ContractWrapper] with contract's `sudo` entry-point and custom message type.
    pub fn with_sudo<T4A, E4A>(
        self,
        sudo_fn: PermissionedFn<T4A, C, E4A, Q>,
    ) -> ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4A, E4A, E5, T6, E6>
    where
        T4A: DeserializeOwned + 'static,
        E4A: Display + Debug + Send + Sync + 'static,
    {
        ContractWrapper {
            execute_fn: self.execute_fn,
            instantiate_fn: self.instantiate_fn,
            query_fn: self.query_fn,
            sudo_fn: Some(Box::new(sudo_fn)),
            reply_fn: self.reply_fn,
            migrate_fn: self.migrate_fn,
            ibc_channel_open_fn: self.ibc_channel_open_fn,
            ibc_channel_connect_fn: self.ibc_channel_connect_fn,
            ibc_channel_close_fn: self.ibc_channel_close_fn,
            ibc_packet_receive_fn: self.ibc_packet_receive_fn,
            ibc_packet_ack_fn: self.ibc_packet_ack_fn,
            ibc_packet_timeout_fn: self.ibc_packet_timeout_fn,
            ibc_source_callback_fn: self.ibc_source_callback_fn,
            ibc_destination_callback_fn: self.ibc_destination_callback_fn,
            checksum: None,
        }
    }

    /// Populates [ContractWrapper] with contract's `sudo` entry-point and `Empty` as a custom message.
    pub fn with_sudo_empty<T4A, E4A>(
        self,
        sudo_fn: PermissionedFn<T4A, Empty, E4A, Empty>,
    ) -> ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4A, E4A, E5, T6, E6>
    where
        T4A: DeserializeOwned + 'static,
        E4A: Display + Debug + Send + Sync + 'static,
    {
        ContractWrapper {
            execute_fn: self.execute_fn,
            instantiate_fn: self.instantiate_fn,
            query_fn: self.query_fn,
            sudo_fn: Some(customize_permissioned_fn(sudo_fn)),
            reply_fn: self.reply_fn,
            migrate_fn: self.migrate_fn,
            ibc_channel_open_fn: self.ibc_channel_open_fn,
            ibc_channel_connect_fn: self.ibc_channel_connect_fn,
            ibc_channel_close_fn: self.ibc_channel_close_fn,
            ibc_packet_receive_fn: self.ibc_packet_receive_fn,
            ibc_packet_ack_fn: self.ibc_packet_ack_fn,
            ibc_packet_timeout_fn: self.ibc_packet_timeout_fn,
            ibc_source_callback_fn: self.ibc_source_callback_fn,
            ibc_destination_callback_fn: self.ibc_destination_callback_fn,
            checksum: None,
        }
    }

    /// Populates [ContractWrapper] with contract's `reply` entry-point and custom message type.
    pub fn with_reply<E5A>(
        self,
        reply_fn: ReplyFn<C, E5A, Q>,
    ) -> ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4, E4, E5A, T6, E6>
    where
        E5A: Display + Debug + Send + Sync + 'static,
    {
        ContractWrapper {
            execute_fn: self.execute_fn,
            instantiate_fn: self.instantiate_fn,
            query_fn: self.query_fn,
            sudo_fn: self.sudo_fn,
            reply_fn: Some(Box::new(reply_fn)),
            migrate_fn: self.migrate_fn,
            ibc_channel_open_fn: self.ibc_channel_open_fn,
            ibc_channel_connect_fn: self.ibc_channel_connect_fn,
            ibc_channel_close_fn: self.ibc_channel_close_fn,
            ibc_packet_receive_fn: self.ibc_packet_receive_fn,
            ibc_packet_ack_fn: self.ibc_packet_ack_fn,
            ibc_packet_timeout_fn: self.ibc_packet_timeout_fn,
            ibc_source_callback_fn: self.ibc_source_callback_fn,
            ibc_destination_callback_fn: self.ibc_destination_callback_fn,
            checksum: None,
        }
    }

    /// Populates [ContractWrapper] with contract's `reply` entry-point and `Empty` as a custom message.
    pub fn with_reply_empty<E5A>(
        self,
        reply_fn: ReplyFn<Empty, E5A, Empty>,
    ) -> ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4, E4, E5A, T6, E6>
    where
        E5A: Display + Debug + Send + Sync + 'static,
    {
        ContractWrapper {
            execute_fn: self.execute_fn,
            instantiate_fn: self.instantiate_fn,
            query_fn: self.query_fn,
            sudo_fn: self.sudo_fn,
            reply_fn: Some(customize_permissioned_fn(reply_fn)),
            migrate_fn: self.migrate_fn,
            ibc_channel_open_fn: self.ibc_channel_open_fn,
            ibc_channel_connect_fn: self.ibc_channel_connect_fn,
            ibc_channel_close_fn: self.ibc_channel_close_fn,
            ibc_packet_receive_fn: self.ibc_packet_receive_fn,
            ibc_packet_ack_fn: self.ibc_packet_ack_fn,
            ibc_packet_timeout_fn: self.ibc_packet_timeout_fn,
            ibc_source_callback_fn: self.ibc_source_callback_fn,
            ibc_destination_callback_fn: self.ibc_destination_callback_fn,
            checksum: None,
        }
    }

    /// Populates [ContractWrapper] with contract's `migrate` entry-point and custom message type.
    pub fn with_migrate<T6A, E6A>(
        self,
        migrate_fn: PermissionedFn<T6A, C, E6A, Q>,
    ) -> ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4, E4, E5, T6A, E6A>
    where
        T6A: DeserializeOwned + 'static,
        E6A: Display + Debug + Send + Sync + 'static,
    {
        ContractWrapper {
            execute_fn: self.execute_fn,
            instantiate_fn: self.instantiate_fn,
            query_fn: self.query_fn,
            sudo_fn: self.sudo_fn,
            reply_fn: self.reply_fn,
            migrate_fn: Some(Box::new(migrate_fn)),
            ibc_channel_open_fn: self.ibc_channel_open_fn,
            ibc_channel_connect_fn: self.ibc_channel_connect_fn,
            ibc_channel_close_fn: self.ibc_channel_close_fn,
            ibc_packet_receive_fn: self.ibc_packet_receive_fn,
            ibc_packet_ack_fn: self.ibc_packet_ack_fn,
            ibc_packet_timeout_fn: self.ibc_packet_timeout_fn,
            ibc_source_callback_fn: self.ibc_source_callback_fn,
            ibc_destination_callback_fn: self.ibc_destination_callback_fn,
            checksum: None,
        }
    }

    /// Populates [ContractWrapper] with contract's `migrate` entry-point and `Empty` as a custom message.
    pub fn with_migrate_empty<T6A, E6A>(
        self,
        migrate_fn: PermissionedFn<T6A, Empty, E6A, Empty>,
    ) -> ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4, E4, E5, T6A, E6A>
    where
        T6A: DeserializeOwned + 'static,
        E6A: Display + Debug + Send + Sync + 'static,
    {
        ContractWrapper {
            execute_fn: self.execute_fn,
            instantiate_fn: self.instantiate_fn,
            query_fn: self.query_fn,
            sudo_fn: self.sudo_fn,
            reply_fn: self.reply_fn,
            migrate_fn: Some(customize_permissioned_fn(migrate_fn)),
            ibc_channel_open_fn: self.ibc_channel_open_fn,
            ibc_channel_connect_fn: self.ibc_channel_connect_fn,
            ibc_channel_close_fn: self.ibc_channel_close_fn,
            ibc_packet_receive_fn: self.ibc_packet_receive_fn,
            ibc_packet_ack_fn: self.ibc_packet_ack_fn,
            ibc_packet_timeout_fn: self.ibc_packet_timeout_fn,
            ibc_source_callback_fn: self.ibc_source_callback_fn,
            ibc_destination_callback_fn: self.ibc_destination_callback_fn,
            checksum: None,
        }
    }

    /// Populates [ContractWrapper] with the provided checksum of the contract's Wasm blob.
    pub fn with_checksum(mut self, checksum: Checksum) -> Self {
        self.checksum = Some(checksum);
        self
    }

    /// Adding IBC capabilities.
    pub fn with_ibc<E7A, E8A, E9A, E10A, E11A, E12A, E13A, E14A>(
        self,
        channel_open_fn: IbcFn<IbcChannelOpenMsg, IbcChannelOpenResponse, E7A, Q>,
        channel_connect_fn: IbcFn<IbcChannelConnectMsg, IbcBasicResponse<C>, E8A, Q>,
        channel_close_fn: IbcFn<IbcChannelCloseMsg, IbcBasicResponse<C>, E9A, Q>,
        ibc_packet_receive_fn: IbcFn<IbcPacketReceiveMsg, IbcReceiveResponse<C>, E10A, Q>,
        ibc_packet_ack_fn: IbcFn<IbcPacketAckMsg, IbcBasicResponse<C>, E11A, Q>,
        ibc_packet_timeout_fn: IbcFn<IbcPacketTimeoutMsg, IbcBasicResponse<C>, E12A, Q>,
        ibc_source_callback_fn: IbcClosure<IbcSourceCallbackMsg, IbcBasicResponse<C>, E13A, Q>,
        ibc_destination_callback_fn: IbcClosure<IbcDestinationCallbackMsg, IbcBasicResponse<C>, E14A, Q>,
    ) -> ContractWrapper<
        T1,
        T2,
        T3,
        E1,
        E2,
        E3,
        C,
        Q,
        T4,
        E4,
        E5,
        T6,
        E6,
        E7A,
        E8A,
        E9A,
        E10A,
        E11A,
        E12A,
        E13A,
        E14A,
    >
    where
        E7A: Display + Debug + Send + Sync + 'static,
        E8A: Display + Debug + Send + Sync + 'static,
        E9A: Display + Debug + Send + Sync + 'static,
        E10A: Display + Debug + Send + Sync + 'static,
        E11A: Display + Debug + Send + Sync + 'static,
        E12A: Display + Debug + Send + Sync + 'static,
        E13A: Display + Debug + Send + Sync + 'static,
        E14A: Display + Debug + Send + Sync + 'static,
    {
        ContractWrapper {
            execute_fn: self.execute_fn,
            instantiate_fn: self.instantiate_fn,
            query_fn: self.query_fn,
            sudo_fn: self.sudo_fn,
            reply_fn: self.reply_fn,
            migrate_fn: self.migrate_fn,
            ibc_channel_open_fn: Some(Box::new(channel_open_fn)),
            ibc_channel_connect_fn: Some(Box::new(channel_connect_fn)),
            ibc_channel_close_fn: Some(Box::new(channel_close_fn)),
            ibc_packet_receive_fn: Some(Box::new(ibc_packet_receive_fn)),
            ibc_packet_ack_fn: Some(Box::new(ibc_packet_ack_fn)),
            ibc_packet_timeout_fn: Some(Box::new(ibc_packet_timeout_fn)),
            ibc_source_callback_fn: Some(Box::new(ibc_source_callback_fn)),
            ibc_destination_callback_fn: Some(Box::new(ibc_destination_callback_fn)),
            checksum: None,
        }
    }
}

fn customize_contract_fn<T, C, E, Q>(
    raw_fn: ContractFn<T, Empty, E, Empty>,
) -> ContractClosure<T, C, E, Q>
where
    T: DeserializeOwned + 'static,
    E: Display + Debug + Send + Sync + 'static,
    C: CustomMsg,
    Q: CustomQuery + DeserializeOwned,
{
    Box::new(
        move |mut deps: DepsMut<Q>,
              env: Env,
              info: MessageInfo,
              msg: T|
              -> Result<Response<C>, E> {
            let deps = decustomize_deps_mut(&mut deps);
            raw_fn(deps, env, info, msg).map(customize_response::<C>)
        },
    )
}

fn customize_query_fn<T, E, Q>(raw_fn: QueryFn<T, E, Empty>) -> QueryClosure<T, E, Q>
where
    T: DeserializeOwned + 'static,
    E: Display + Debug + Send + Sync + 'static,
    Q: CustomQuery + DeserializeOwned,
{
    Box::new(
        move |deps: Deps<Q>, env: Env, msg: T| -> Result<Binary, E> {
            let deps = decustomize_deps(&deps);
            raw_fn(deps, env, msg)
        },
    )
}

fn customize_permissioned_fn<T, C, E, Q>(
    raw_fn: PermissionedFn<T, Empty, E, Empty>,
) -> PermissionedClosure<T, C, E, Q>
where
    T: DeserializeOwned + 'static,
    E: Display + Debug + Send + Sync + 'static,
    C: CustomMsg,
    Q: CustomQuery + DeserializeOwned,
{
    Box::new(
        move |mut deps: DepsMut<Q>, env: Env, msg: T| -> Result<Response<C>, E> {
            let deps = decustomize_deps_mut(&mut deps);
            raw_fn(deps, env, msg).map(customize_response::<C>)
        },
    )
}

fn decustomize_deps_mut<'a, Q>(deps: &'a mut DepsMut<Q>) -> DepsMut<'a, Empty>
where
    Q: CustomQuery + DeserializeOwned,
{
    DepsMut {
        storage: deps.storage,
        api: deps.api,
        querier: QuerierWrapper::new(deps.querier.deref()),
    }
}

fn decustomize_deps<'a, Q>(deps: &'a Deps<'a, Q>) -> Deps<'a, Empty>
where
    Q: CustomQuery + DeserializeOwned,
{
    Deps {
        storage: deps.storage,
        api: deps.api,
        querier: QuerierWrapper::new(deps.querier.deref()),
    }
}

fn customize_response<C>(resp: Response<Empty>) -> Response<C>
where
    C: CustomMsg,
{
    let mut customized_resp = Response::<C>::new()
        .add_submessages(resp.messages.into_iter().map(customize_msg::<C>))
        .add_events(resp.events)
        .add_attributes(resp.attributes);
    customized_resp.data = resp.data;
    customized_resp
}

fn customize_msg<C>(msg: SubMsg<Empty>) -> SubMsg<C>
where
    C: CustomMsg,
{
    SubMsg {
        id: msg.id,
        payload: msg.payload,
        msg: match msg.msg {
            CosmosMsg::Wasm(wasm) => CosmosMsg::Wasm(wasm),
            CosmosMsg::Bank(bank) => CosmosMsg::Bank(bank),
            #[cfg(feature = "staking")]
            CosmosMsg::Staking(staking) => CosmosMsg::Staking(staking),
            #[cfg(feature = "staking")]
            CosmosMsg::Distribution(distribution) => CosmosMsg::Distribution(distribution),
            CosmosMsg::Custom(_) => unreachable!(),
            #[cfg(feature = "stargate")]
            CosmosMsg::Ibc(ibc) => CosmosMsg::Ibc(ibc),
            #[cfg(feature = "cosmwasm_2_0")]
            CosmosMsg::Any(any) => CosmosMsg::Any(any),
            other => panic!("unknown message variant {:?}", other),
        },
        gas_limit: msg.gas_limit,
        reply_on: msg.reply_on,
    }
}

impl<T1, T2, T3, E1, E2, E3, C, T4, E4, E5, T6, E6, E7, E8, E9, E10, E11, E12, E13, E14, Q>
    Contract<C, Q>
    for ContractWrapper<
        T1,
        T2,
        T3,
        E1,
        E2,
        E3,
        C,
        Q,
        T4,
        E4,
        E5,
        T6,
        E6,
        E7,
        E8,
        E9,
        E10,
        E11,
        E12,
        E13,
        E14,
    >
where
    T1: DeserializeOwned, // Type of message passed to `execute` entry-point.
    T2: DeserializeOwned, // Type of message passed to `instantiate` entry-point.
    T3: DeserializeOwned, // Type of message passed to `query` entry-point.
    T4: DeserializeOwned, // Type of message passed to `sudo` entry-point.
    T6: DeserializeOwned, // Type of message passed to `migrate` entry-point.
    E1: Display + Debug + Send + Sync + 'static, // Type of error returned from `execute` entry-point.
    E2: Display + Debug + Send + Sync + 'static, // Type of error returned from `instantiate` entry-point.
    E3: Display + Debug + Send + Sync + 'static, // Type of error returned from `query` entry-point.
    E4: Display + Debug + Send + Sync + 'static, // Type of error returned from `sudo` entry-point.
    E5: Display + Debug + Send + Sync + 'static, // Type of error returned from `reply` entry-point.
    E6: Display + Debug + Send + Sync + 'static, // Type of error returned from `migrate` entry-point.
    E7: Display + Debug + Send + Sync + 'static, // Type of error returned from `channel_open` entry-point.
    E8: Display + Debug + Send + Sync + 'static, // Type of error returned from `channel_connect` entry-point.
    E9: Display + Debug + Send + Sync + 'static, // Type of error returned from `channel_close` entry-point.
    E10: Display + Debug + Send + Sync + 'static, // Type of error returned from `ibc_packet_receive` entry-point.
    E11: Display + Debug + Send + Sync + 'static, // Type of error returned from `ibc_packet_ack` entry-point.
    E12: Display + Debug + Send + Sync + 'static, // Type of error returned from `ibc_packet_timeout` entry-point.
    E13: Display + Debug + Send + Sync + 'static, // Type of error returned from `ibc_source_callback` entry-point.
    E14: Display + Debug + Send + Sync + 'static, // Type of error returned from `ibc_destination_callback` entry-point.
    C: CustomMsg, // Type of custom message returned from all entry-points except `query`.
    Q: CustomQuery + DeserializeOwned, // Type of custom query in querier passed as deps/deps_mut to all entry-points.
{
    /// Calls [execute] on wrapped [Contract] trait implementor.
    ///
    /// [execute]: Contract::execute
    fn execute(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<C>> {
        let msg: T1 = from_json(msg)?;
        (self.execute_fn)(deps, env, info, msg).map_err(|err: E1| anyhow!(err))
    }

    /// Calls [instantiate] on wrapped [Contract] trait implementor.
    ///
    /// [instantiate]: Contract::instantiate
    fn instantiate(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<C>> {
        let msg: T2 = from_json(msg)?;
        (self.instantiate_fn)(deps, env, info, msg).map_err(|err: E2| anyhow!(err))
    }

    /// Calls [query] on wrapped [Contract] trait implementor.
    ///
    /// [query]: Contract::query
    fn query(&self, deps: Deps<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Binary> {
        let msg: T3 = from_json(msg)?;
        (self.query_fn)(deps, env, msg).map_err(|err: E3| anyhow!(err))
    }

    /// Calls [sudo] on wrapped [Contract] trait implementor.
    /// Returns an error when the contract does not implement [sudo].
    ///
    /// [sudo]: Contract::sudo
    fn sudo(&self, deps: DepsMut<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Response<C>> {
        let msg: T4 = from_json(msg)?;
        match &self.sudo_fn {
            Some(sudo) => sudo(deps, env, msg).map_err(|err: E4| anyhow!(err)),
            None => bail!("sudo is not implemented for contract"),
        }
    }

    /// Calls [reply] on wrapped [Contract] trait implementor.
    /// Returns an error when the contract does not implement [reply].
    ///
    /// [reply]: Contract::reply
    fn reply(&self, deps: DepsMut<Q>, env: Env, reply_data: Reply) -> AnyResult<Response<C>> {
        let msg: Reply = reply_data;
        match &self.reply_fn {
            Some(reply) => reply(deps, env, msg).map_err(|err: E5| anyhow!(err)),
            None => bail!("reply is not implemented for contract"),
        }
    }

    /// Calls [migrate] on wrapped [Contract] trait implementor.
    /// Returns an error when the contract does not implement [migrate].
    ///
    /// [migrate]: Contract::migrate
    fn migrate(&self, deps: DepsMut<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Response<C>> {
        let msg: T6 = from_json(msg)?;
        match &self.migrate_fn {
            Some(migrate) => migrate(deps, env, msg).map_err(|err: E6| anyhow!(err)),
            None => bail!("migrate is not implemented for contract"),
        }
    }

    /// Calls [ibc_channel_open] on wrapped [Contract] trait implementor.
    /// Returns an error when the contract does not implement [ibc_channel_open].
    ///
    /// [ibc_channel_open]: Contract::ibc_channel_open
    fn ibc_channel_open(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        msg: IbcChannelOpenMsg,
    ) -> AnyResult<IbcChannelOpenResponse> {
        match &self.ibc_channel_open_fn {
            Some(channel_open) => channel_open(deps, env, msg).map_err(|err: E7| anyhow!(err)),
            None => bail!("ibc_channel_open is not implemented for contract"),
        }
    }

    /// Calls [ibc_channel_connect] on wrapped [Contract] trait implementor.
    /// Returns an error when the contract does not implement [ibc_channel_connect].
    ///
    /// [ibc_channel_connect]: Contract::ibc_channel_connect
    fn ibc_channel_connect(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        msg: IbcChannelConnectMsg,
    ) -> AnyResult<IbcBasicResponse<C>> {
        match &self.ibc_channel_connect_fn {
            Some(channel_connect) => {
                channel_connect(deps, env, msg).map_err(|err: E8| anyhow!(err))
            }
            None => bail!("ibc_channel_connect is not implemented for contract"),
        }
    }

    /// Calls [ibc_channel_close] on wrapped [Contract] trait implementor.
    /// Returns an error when the contract does not implement [ibc_channel_close].
    ///
    /// [ibc_channel_close]: Contract::ibc_channel_close
    fn ibc_channel_close(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        msg: IbcChannelCloseMsg,
    ) -> AnyResult<IbcBasicResponse<C>> {
        match &self.ibc_channel_close_fn {
            Some(channel_close) => channel_close(deps, env, msg).map_err(|err: E9| anyhow!(err)),
            None => bail!("ibc_channel_close is not implemented for contract"),
        }
    }

    /// Calls [ibc_packet_receive] on wrapped [Contract] trait implementor.
    /// Returns an error when the contract does not implement [ibc_packet_receive].
    ///
    /// [ibc_packet_receive]: Contract::ibc_packet_receive
    fn ibc_packet_receive(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        msg: IbcPacketReceiveMsg,
    ) -> AnyResult<IbcReceiveResponse<C>> {
        match &self.ibc_packet_receive_fn {
            Some(packet_receive) => packet_receive(deps, env, msg).map_err(|err: E10| anyhow!(err)),
            None => bail!("ibc_packet_receive is not implemented for contract"),
        }
    }

    /// Calls [ibc_packet_acknowledge] on wrapped [Contract] trait implementor.
    /// Returns an error when the contract does not implement [ibc_packet_acknowledge].
    ///
    /// [ibc_packet_acknowledge]: Contract::ibc_packet_ack
    fn ibc_packet_ack(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        msg: IbcPacketAckMsg,
    ) -> AnyResult<IbcBasicResponse<C>> {
        match &self.ibc_packet_ack_fn {
            Some(packet_ack) => packet_ack(deps, env, msg).map_err(|err: E11| anyhow!(err)),
            None => bail!("ibc_packet_acknowledge is not implemented for contract"),
        }
    }

    /// Calls [ibc_packet_timeout] on wrapped [Contract] trait implementor.
    /// Returns an error when the contract does not implement [ibc_packet_timeout].
    ///
    /// [ibc_packet_timeout]: Contract::ibc_packet_timeout
    fn ibc_packet_timeout(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        msg: IbcPacketTimeoutMsg,
    ) -> AnyResult<IbcBasicResponse<C>> {
        match &self.ibc_packet_timeout_fn {
            Some(packet_timeout) => packet_timeout(deps, env, msg).map_err(|err: E12| anyhow!(err)),
            None => bail!("ibc_packet_timeout is not implemented for contract"),
        }
    }

    /// Calls [ibc_source_callback] on wrapped [Contract] trait implementor.
    /// Returns an error when the contract does not implement [ibc_source_callback].
    ///
    /// [ibc_source_callback]: Contract::ibc_source_callback
    fn ibc_source_callback(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        msg: IbcSourceCallbackMsg,
    ) -> AnyResult<IbcBasicResponse<C>> {
        match &self.ibc_source_callback_fn {
            Some(source_callback) => {
                source_callback(deps, env, msg).map_err(|err: E13| anyhow!(err))
            }
            None => bail!("ibc_source_callback is not implemented for contract"),
        }
    }

    /// Calls [ibc_destination_callback] on wrapped [Contract] trait implementor.
    /// Returns an error when the contract does not implement [ibc_destination_callback].
    ///
    /// [ibc_destination_callback]: Contract::ibc_destination_callback
    fn ibc_destination_callback(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        msg: IbcDestinationCallbackMsg,
    ) -> AnyResult<IbcBasicResponse<C>> {
        match &self.ibc_destination_callback_fn {
            Some(destination_callback) => {
                destination_callback(deps, env, msg).map_err(|err: E14| anyhow!(err))
            }
            None => bail!("ibc_destination_callback is not implemented for contract"),
        }
    }

    /// Returns the provided checksum of the contract's `wasm` blob.
    fn checksum(&self) -> Option<Checksum> {
        self.checksum
    }
}
