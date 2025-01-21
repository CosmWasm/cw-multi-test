mod test_bank_responses;

mod test_contracts {

    /// Example smart contract for testing submessage responses.
    pub mod responder {
        use cosmwasm_schema::{cw_serde, QueryResponses};
        use cosmwasm_std::{
            to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty, Env,
            MessageInfo, MsgResponse, Reply, ReplyOn, Response, StdResult, SubMsg, SubMsgResponse,
            SubMsgResult, Uint128, WasmMsg,
        };

        /// Messages instantiating the contract.
        #[cw_serde]
        pub enum ResponderInstantiateMessage {
            None,
        }

        /// Messages executed by the contract.
        #[cw_serde]
        pub enum ResponderExecuteMessage {
            /// Adds two unsigned integers and returns the sum.
            Add(u64, u64),
            /// Returns submessage `WasmMsg::Execute` with `Add` message.
            WasmMsgExecuteAdd(String, u64, u64),
            /// Returns submessage `BankMsg::Send`.
            BankSend(String, u128, String),
            /// Returns submessage `BankMsg::Burn`.
            BankBurn(u128, String),
        }

        /// Messages querying the contract.
        #[cw_serde]
        #[derive(QueryResponses)]
        pub enum ResponderQueryMessage {
            #[returns(ResponderResponse)]
            Value,
        }

        /// Utility structure for convenient data transfer
        /// from reply entry-point back to caller.
        #[cw_serde]
        pub struct ResponderResponse {
            pub id: Option<u64>,
            pub msg_responses: Vec<MsgResponse>,
        }

        /// Entry-point for instantiating the contract.
        pub fn instantiate(
            _deps: DepsMut,
            _env: Env,
            _info: MessageInfo,
            _msg: ResponderInstantiateMessage,
        ) -> StdResult<Response> {
            Ok(Response::default())
        }

        /// Entry-point for executing contract's messages.
        pub fn execute(
            _deps: DepsMut,
            _env: Env,
            _info: MessageInfo,
            msg: ResponderExecuteMessage,
        ) -> StdResult<Response> {
            Ok(match msg {
                ResponderExecuteMessage::Add(value1, value2) => {
                    let sum = value1.saturating_add(value2);
                    Response::new().set_data(to_json_binary(&sum)?)
                }
                ResponderExecuteMessage::WasmMsgExecuteAdd(contract_addr, value1, value2) => {
                    let msg = ResponderExecuteMessage::Add(value1, value2);
                    Response::new().add_submessage(reply_always(
                        3,
                        WasmMsg::Execute {
                            contract_addr,
                            msg: to_json_binary(&msg)?,
                            funds: vec![],
                        }
                        .into(),
                    ))
                }
                ResponderExecuteMessage::BankSend(addr, amount, denom) => Response::new()
                    .add_submessage(reply_always(
                        1,
                        BankMsg::Send {
                            to_address: addr.clone(),
                            amount: coins(amount, denom),
                        }
                        .into(),
                    )),
                ResponderExecuteMessage::BankBurn(amount, denom) => {
                    Response::new().add_submessage(reply_always(
                        2,
                        BankMsg::Burn {
                            amount: coins(amount, denom),
                        }
                        .into(),
                    ))
                }
            })
        }

        /// Entry-point for querying the contract.
        pub fn query(_deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
            Ok(Binary::default())
        }

        /// Entry-point for handling submessage replies.
        pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
            #[allow(deprecated)]
            if let Reply {
                id,
                result:
                    SubMsgResult::Ok(SubMsgResponse {
                        events: _,
                        data: _,
                        msg_responses,
                    }),
                ..
            } = msg
            {
                Ok(Response::new().set_data(to_json_binary(&ResponderResponse {
                    id: Some(id),
                    msg_responses,
                })?))
            } else {
                Ok(Response::new().set_data(to_json_binary(&ResponderResponse {
                    id: None,
                    msg_responses: vec![],
                })?))
            }
        }

        /// Utility function for creating coins.
        fn coins(amount: u128, denom: String) -> Vec<Coin> {
            vec![Coin::new(Uint128::new(amount), denom.clone())]
        }

        /// Utility function for creating submessages that require reply.
        fn reply_always(id: u64, msg: CosmosMsg) -> SubMsg {
            SubMsg {
                id,
                payload: Default::default(),
                msg,
                gas_limit: None,
                reply_on: ReplyOn::Always,
            }
        }
    }
}
