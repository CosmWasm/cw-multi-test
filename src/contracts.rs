#![allow(clippy::type_complexity)]

use crate::error::{anyhow, bail, AnyError, AnyResult};
use cosmwasm_std::{
    from_json, Binary, CosmosMsg, CustomMsg, CustomQuery, Deps, DepsMut, Empty, Env, MessageInfo,
    QuerierWrapper, Reply, Response, SubMsg,
};
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt::{Debug, Display};
use std::ops::Deref;

/// Serves as the primary interface for interacting with contracts.
///
/// It includes methods for executing, querying, and managing contract states,
/// making it a fundamental trait for testing contracts.
pub trait Contract<T, Q = Empty>
where
    T: CustomMsg,
    Q: CustomQuery,
{
    /// Evaluates contract's `execute` entry-point.
    fn execute(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<T>>;

    /// Evaluates contract's `instantiate` entry-point.
    fn instantiate(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<T>>;

    /// Evaluates contract's `query` entry-point.
    fn query(&self, deps: Deps<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Binary>;

    /// Evaluates contract's `sudo` entry-point.
    fn sudo(&self, deps: DepsMut<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Response<T>>;

    /// Evaluates contract's `reply` entry-point.
    fn reply(&self, deps: DepsMut<Q>, env: Env, msg: Reply) -> AnyResult<Response<T>>;

    /// Evaluates contract's `migrate` entry-point.
    fn migrate(&self, deps: DepsMut<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Response<T>>;
}

type ContractFn<T, C, E, Q> =
    fn(deps: DepsMut<Q>, env: Env, info: MessageInfo, msg: T) -> Result<Response<C>, E>;
type PermissionedFn<T, C, E, Q> = fn(deps: DepsMut<Q>, env: Env, msg: T) -> Result<Response<C>, E>;
type ReplyFn<C, E, Q> = fn(deps: DepsMut<Q>, env: Env, msg: Reply) -> Result<Response<C>, E>;
type QueryFn<T, E, Q> = fn(deps: Deps<Q>, env: Env, msg: T) -> Result<Binary, E>;

type ContractClosure<T, C, E, Q> =
    Box<dyn Fn(DepsMut<Q>, Env, MessageInfo, T) -> Result<Response<C>, E>>;
type PermissionedClosure<T, C, E, Q> = Box<dyn Fn(DepsMut<Q>, Env, T) -> Result<Response<C>, E>>;
type ReplyClosure<C, E, Q> = Box<dyn Fn(DepsMut<Q>, Env, Reply) -> Result<Response<C>, E>>;
type QueryClosure<T, E, Q> = Box<dyn Fn(Deps<Q>, Env, T) -> Result<Binary, E>>;

/// Standardizes interactions with contracts in CosmWasm tests, especially useful for contracts that
/// do not possess extensive privileges. It simplifies and unifies the way developers interact with
/// different contracts.
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
> where
    T1: DeserializeOwned + Debug,
    T2: DeserializeOwned,
    T3: DeserializeOwned,
    T4: DeserializeOwned,
    T6: DeserializeOwned,
    E1: Display + Debug + Send + Sync + 'static,
    E2: Display + Debug + Send + Sync + 'static,
    E3: Display + Debug + Send + Sync + 'static,
    E4: Display + Debug + Send + Sync + 'static,
    E5: Display + Debug + Send + Sync + 'static,
    E6: Display + Debug + Send + Sync + 'static,
    C: CustomMsg,
    Q: CustomQuery + DeserializeOwned + 'static,
{
    execute_fn: ContractClosure<T1, C, E1, Q>,
    instantiate_fn: ContractClosure<T2, C, E2, Q>,
    query_fn: QueryClosure<T3, E3, Q>,
    sudo_fn: Option<PermissionedClosure<T4, C, E4, Q>>,
    reply_fn: Option<ReplyClosure<C, E5, Q>>,
    migrate_fn: Option<PermissionedClosure<T6, C, E6, Q>>,
}

impl<T1, T2, T3, E1, E2, E3, C, Q> ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q>
where
    T1: DeserializeOwned + Debug + 'static,
    T2: DeserializeOwned + 'static,
    T3: DeserializeOwned + 'static,
    E1: Display + Debug + Send + Sync + 'static,
    E2: Display + Debug + Send + Sync + 'static,
    E3: Display + Debug + Send + Sync + 'static,
    C: CustomMsg + 'static,
    Q: CustomQuery + DeserializeOwned + 'static,
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
        }
    }

    /// This will take a contract that returns `Response<Empty>` and will "upgrade" it
    /// to `Response<C>` if needed to be compatible with a chain-specific extension.
    pub fn new_with_empty(
        execute_fn: ContractFn<T1, Empty, E1, Empty>,
        instantiate_fn: ContractFn<T2, Empty, E2, Empty>,
        query_fn: QueryFn<T3, E3, Empty>,
    ) -> Self {
        Self {
            execute_fn: customize_fn(execute_fn),
            instantiate_fn: customize_fn(instantiate_fn),
            query_fn: customize_query(query_fn),
            sudo_fn: None,
            reply_fn: None,
            migrate_fn: None,
        }
    }
}

impl<T1, T2, T3, E1, E2, E3, C, Q, T4, E4, E5, T6, E6>
    ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4, E4, E5, T6, E6>
where
    T1: DeserializeOwned + Debug + 'static,
    T2: DeserializeOwned + 'static,
    T3: DeserializeOwned + 'static,
    T4: DeserializeOwned + 'static,
    T6: DeserializeOwned + 'static,
    E1: Display + Debug + Send + Sync + 'static,
    E2: Display + Debug + Send + Sync + 'static,
    E3: Display + Debug + Send + Sync + 'static,
    E4: Display + Debug + Send + Sync + 'static,
    E5: Display + Debug + Send + Sync + 'static,
    E6: Display + Debug + Send + Sync + 'static,
    C: CustomMsg + 'static,
    Q: CustomQuery + DeserializeOwned + 'static,
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
        }
    }
}

fn customize_fn<T, C, E, Q>(raw_fn: ContractFn<T, Empty, E, Empty>) -> ContractClosure<T, C, E, Q>
where
    T: DeserializeOwned + 'static,
    E: Display + Debug + Send + Sync + 'static,
    C: CustomMsg + 'static,
    Q: CustomQuery + DeserializeOwned + 'static,
{
    let customized = move |mut deps: DepsMut<Q>,
                           env: Env,
                           info: MessageInfo,
                           msg: T|
          -> Result<Response<C>, E> {
        let deps = decustomize_deps_mut(&mut deps);
        raw_fn(deps, env, info, msg).map(customize_response::<C>)
    };
    Box::new(customized)
}

fn customize_query<T, E, Q>(raw_fn: QueryFn<T, E, Empty>) -> QueryClosure<T, E, Q>
where
    T: DeserializeOwned + 'static,
    E: Display + Debug + Send + Sync + 'static,
    Q: CustomQuery + DeserializeOwned + 'static,
{
    let customized = move |deps: Deps<Q>, env: Env, msg: T| -> Result<Binary, E> {
        let deps = decustomize_deps(&deps);
        raw_fn(deps, env, msg)
    };
    Box::new(customized)
}

fn decustomize_deps_mut<'a, Q>(deps: &'a mut DepsMut<Q>) -> DepsMut<'a, Empty>
where
    Q: CustomQuery + DeserializeOwned + 'static,
{
    DepsMut {
        storage: deps.storage,
        api: deps.api,
        querier: QuerierWrapper::new(deps.querier.deref()),
    }
}

fn decustomize_deps<'a, Q>(deps: &'a Deps<'a, Q>) -> Deps<'a, Empty>
where
    Q: CustomQuery + DeserializeOwned + 'static,
{
    Deps {
        storage: deps.storage,
        api: deps.api,
        querier: QuerierWrapper::new(deps.querier.deref()),
    }
}

fn customize_permissioned_fn<T, C, E, Q>(
    raw_fn: PermissionedFn<T, Empty, E, Empty>,
) -> PermissionedClosure<T, C, E, Q>
where
    T: DeserializeOwned + 'static,
    E: Display + Debug + Send + Sync + 'static,
    C: CustomMsg + 'static,
    Q: CustomQuery + DeserializeOwned + 'static,
{
    let customized = move |mut deps: DepsMut<Q>, env: Env, msg: T| -> Result<Response<C>, E> {
        let deps = decustomize_deps_mut(&mut deps);
        raw_fn(deps, env, msg).map(customize_response::<C>)
    };
    Box::new(customized)
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
        msg: match msg.msg {
            CosmosMsg::Wasm(wasm) => CosmosMsg::Wasm(wasm),
            CosmosMsg::Bank(bank) => CosmosMsg::Bank(bank),
            CosmosMsg::Staking(staking) => CosmosMsg::Staking(staking),
            CosmosMsg::Distribution(distribution) => CosmosMsg::Distribution(distribution),
            CosmosMsg::Custom(_) => unreachable!(),
            CosmosMsg::Ibc(ibc) => CosmosMsg::Ibc(ibc),
            CosmosMsg::Stargate { type_url, value } => CosmosMsg::Stargate { type_url, value },
            _ => panic!("unknown message variant {:?}", msg),
        },
        id: msg.id,
        gas_limit: msg.gas_limit,
        reply_on: msg.reply_on,
    }
}

impl<T1, T2, T3, E1, E2, E3, C, T4, E4, E5, T6, E6, Q> Contract<C, Q>
    for ContractWrapper<T1, T2, T3, E1, E2, E3, C, Q, T4, E4, E5, T6, E6>
where
    T1: DeserializeOwned + Debug + Clone,
    T2: DeserializeOwned + Debug + Clone,
    T3: DeserializeOwned + Debug + Clone,
    T4: DeserializeOwned,
    T6: DeserializeOwned,
    E1: Display + Debug + Send + Sync + Error + 'static,
    E2: Display + Debug + Send + Sync + Error + 'static,
    E3: Display + Debug + Send + Sync + Error + 'static,
    E4: Display + Debug + Send + Sync + 'static,
    E5: Display + Debug + Send + Sync + 'static,
    E6: Display + Debug + Send + Sync + 'static,
    C: CustomMsg,
    Q: CustomQuery + DeserializeOwned,
{
    fn execute(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<C>> {
        let msg: T1 = from_json(msg)?;
        (self.execute_fn)(deps, env, info, msg).map_err(|err| anyhow!(err))
    }

    fn instantiate(
        &self,
        deps: DepsMut<Q>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<C>> {
        let msg: T2 = from_json(msg)?;
        (self.instantiate_fn)(deps, env, info, msg).map_err(|err| anyhow!(err))
    }

    fn query(&self, deps: Deps<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Binary> {
        let msg: T3 = from_json(msg)?;
        (self.query_fn)(deps, env, msg).map_err(|err| anyhow!(err))
    }

    // this returns an error if the contract doesn't implement sudo
    fn sudo(&self, deps: DepsMut<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Response<C>> {
        let msg = from_json(msg)?;
        match &self.sudo_fn {
            Some(sudo) => sudo(deps, env, msg).map_err(|err| anyhow!(err)),
            None => bail!("sudo not implemented for contract"),
        }
    }

    // this returns an error if the contract doesn't implement reply
    fn reply(&self, deps: DepsMut<Q>, env: Env, reply_data: Reply) -> AnyResult<Response<C>> {
        match &self.reply_fn {
            Some(reply) => reply(deps, env, reply_data).map_err(|err| anyhow!(err)),
            None => bail!("reply not implemented for contract"),
        }
    }

    // this returns an error if the contract doesn't implement migrate
    fn migrate(&self, deps: DepsMut<Q>, env: Env, msg: Vec<u8>) -> AnyResult<Response<C>> {
        let msg = from_json(msg)?;
        match &self.migrate_fn {
            Some(migrate) => migrate(deps, env, msg).map_err(|err| anyhow!(err)),
            None => bail!("migrate not implemented for contract"),
        }
    }
}
