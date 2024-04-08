use crate::error::AnyResult;
use cosmwasm_std::{
    to_json_binary, Addr, Attribute, BankMsg, Binary, Coin, CosmosMsg, CustomMsg, Event,
    SubMsgResponse, WasmMsg,
};
use cw_utils::{parse_execute_response_data, parse_instantiate_response_data};
use serde::Serialize;
use std::fmt::Debug;

/// A subset of data returned as a response of a contract entry point,
/// such as `instantiate`, `execute` or `migrate`.
#[derive(Default, Clone, Debug)]
pub struct AppResponse {
    /// Response events.
    pub events: Vec<Event>,
    /// Response data.
    pub data: Option<Binary>,
}

impl AppResponse {
    /// Returns all custom attributes returned by the contract in the `idx` event.
    ///
    /// We assert the type is wasm, and skip the contract_address attribute.
    #[track_caller]
    pub fn custom_attrs(&self, idx: usize) -> &[Attribute] {
        assert_eq!(self.events[idx].ty.as_str(), "wasm");
        &self.events[idx].attributes[1..]
    }

    /// Checks if there is an Event that is a super-set of this.
    ///
    /// It has the same type, and all compared attributes are included in it as well.
    /// You don't need to specify them all.
    pub fn has_event(&self, expected: &Event) -> bool {
        self.events.iter().any(|ev| {
            expected.ty == ev.ty
                && expected
                    .attributes
                    .iter()
                    .all(|at| ev.attributes.contains(at))
        })
    }

    /// Like [has_event](Self::has_event) but panics if there is no match.
    #[track_caller]
    pub fn assert_event(&self, expected: &Event) {
        assert!(
            self.has_event(expected),
            "Expected to find an event {:?}, but received: {:?}",
            expected,
            self.events
        );
    }
}

/// They have the same shape, SubMsgResponse is what is returned in reply.
/// This is just to make some test cases easier.
impl From<SubMsgResponse> for AppResponse {
    fn from(reply: SubMsgResponse) -> Self {
        AppResponse {
            data: reply.data,
            events: reply.events,
        }
    }
}
/// A trait defining a default behavior of the message executor.
///
/// Defines the interface for executing transactions and contract interactions.
/// It is a central component in the testing framework, managing the operational
/// flow and ensuring that contract _calls_ are processed correctly.
pub trait Executor<C>
where
    C: CustomMsg + 'static,
{
    /// Processes (executes) an arbitrary `CosmosMsg`.
    /// This will create a cache before the execution,
    /// so no state changes are persisted if this returns an error,
    /// but all are persisted on success.
    fn execute(&mut self, sender: Addr, msg: CosmosMsg<C>) -> AnyResult<AppResponse>;

    /// Create a contract and get the new address.
    /// This is just a helper around execute()
    fn instantiate_contract<T: Serialize, U: Into<String>>(
        &mut self,
        code_id: u64,
        sender: Addr,
        init_msg: &T,
        send_funds: &[Coin],
        label: U,
        admin: Option<String>,
    ) -> AnyResult<Addr> {
        // instantiate contract
        let init_msg = to_json_binary(init_msg)?;
        let msg = WasmMsg::Instantiate {
            admin,
            code_id,
            msg: init_msg,
            funds: send_funds.to_vec(),
            label: label.into(),
        };
        let res = self.execute(sender, msg.into())?;
        let data = parse_instantiate_response_data(res.data.unwrap_or_default().as_slice())?;
        Ok(Addr::unchecked(data.contract_address))
    }

    /// Instantiates a new contract and returns its predictable address.
    /// This is a helper function around [execute][Self::execute] function
    /// with `WasmMsg::Instantiate2` message.
    #[cfg(feature = "cosmwasm_1_2")]
    fn instantiate2_contract<M, L, A, S>(
        &mut self,
        code_id: u64,
        sender: Addr,
        init_msg: &M,
        funds: &[Coin],
        label: L,
        admin: A,
        salt: S,
    ) -> AnyResult<Addr>
    where
        M: Serialize,
        L: Into<String>,
        A: Into<Option<String>>,
        S: Into<Binary>,
    {
        let msg = WasmMsg::Instantiate2 {
            admin: admin.into(),
            code_id,
            msg: to_json_binary(init_msg)?,
            funds: funds.to_vec(),
            label: label.into(),
            salt: salt.into(),
        };
        let execute_response = self.execute(sender, msg.into())?;
        let instantiate_response =
            parse_instantiate_response_data(execute_response.data.unwrap_or_default().as_slice())?;
        Ok(Addr::unchecked(instantiate_response.contract_address))
    }

    /// Execute a contract and process all returned messages.
    /// This is just a helper function around [execute()](Self::execute)
    /// with `WasmMsg::Execute` message, but in this case we parse out the data field
    /// to that what is returned by the contract (not the protobuf wrapper).
    fn execute_contract<T: Serialize + Debug>(
        &mut self,
        sender: Addr,
        contract_addr: Addr,
        msg: &T,
        send_funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        let binary_msg = to_json_binary(msg)?;
        let wrapped_msg = WasmMsg::Execute {
            contract_addr: contract_addr.into_string(),
            msg: binary_msg,
            funds: send_funds.to_vec(),
        };
        let mut res = self.execute(sender, wrapped_msg.into())?;
        res.data = res
            .data
            .and_then(|d| parse_execute_response_data(d.as_slice()).unwrap().data);
        Ok(res)
    }

    /// Migrates a contract.
    /// Sender must be registered admin.
    /// This is just a helper function around [execute()](Self::execute)
    /// with `WasmMsg::Migrate` message.
    fn migrate_contract<T: Serialize>(
        &mut self,
        sender: Addr,
        contract_addr: Addr,
        msg: &T,
        new_code_id: u64,
    ) -> AnyResult<AppResponse> {
        let msg = to_json_binary(msg)?;
        let msg = WasmMsg::Migrate {
            contract_addr: contract_addr.into(),
            msg,
            new_code_id,
        };
        self.execute(sender, msg.into())
    }

    /// Sends tokens to specified recipient.
    /// This is just a helper function around [execute()](Self::execute)
    /// with `BankMsg::Send` message.
    fn send_tokens(
        &mut self,
        sender: Addr,
        recipient: Addr,
        amount: &[Coin],
    ) -> AnyResult<AppResponse> {
        let msg = BankMsg::Send {
            to_address: recipient.to_string(),
            amount: amount.to_vec(),
        };
        self.execute(sender, msg.into())
    }
}
