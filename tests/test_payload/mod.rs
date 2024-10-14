mod test_submessage_payload;

mod test_contracts {
    pub mod payloader {
        use cosmwasm_schema::cw_serde;
        use cosmwasm_std::{
            from_json, to_json_binary, BankMsg, Binary, Coin, Deps, DepsMut, Empty, Env,
            MessageInfo, Reply, ReplyOn, Response, StdResult, SubMsg, Uint128,
        };

        #[cw_serde]
        pub enum ExecuteMessage {
            Send(String, u128, String),
            SendMulti(String, u128, String, u128, String),
            Burn(u128, String),
            BurnNoPayload(u128, String),
            Nop,
        }

        #[cw_serde]
        pub struct Payload {
            pub id: u64,
            pub action: String,
        }

        pub fn instantiate(
            _deps: DepsMut,
            _env: Env,
            _info: MessageInfo,
            _msg: Empty,
        ) -> StdResult<Response> {
            Ok(Response::default())
        }

        pub fn execute(
            _deps: DepsMut,
            _env: Env,
            _info: MessageInfo,
            msg: ExecuteMessage,
        ) -> StdResult<Response> {
            let mut response = Response::new().set_data(to_json_binary(&Payload {
                id: 0,
                action: "EXECUTE".to_string(),
            })?);
            match msg {
                ExecuteMessage::Send(addr, amount, denom) => {
                    let msg_send = BankMsg::Send {
                        to_address: addr.clone(),
                        amount: vec![Coin::new(Uint128::new(amount), denom.clone())],
                    };
                    response = response.add_submessage(SubMsg {
                        id: 1,
                        payload: to_json_binary(&Payload {
                            id: 0,
                            action: "SEND".to_string(),
                        })?,
                        msg: msg_send.into(),
                        gas_limit: None,
                        reply_on: ReplyOn::Always,
                    });
                }
                ExecuteMessage::SendMulti(addr1, amount1, addr2, amount2, denom) => {
                    let msg_send = BankMsg::Send {
                        to_address: addr1.clone(),
                        amount: vec![Coin::new(Uint128::new(amount1), denom.clone())],
                    };
                    response = response.add_submessage(SubMsg {
                        id: 2,
                        payload: to_json_binary(&Payload {
                            id: 0,
                            action: "SEND".to_string(),
                        })?,
                        msg: msg_send.into(),
                        gas_limit: None,
                        reply_on: ReplyOn::Always,
                    });
                    let msg_send = BankMsg::Send {
                        to_address: addr2.clone(),
                        amount: vec![Coin::new(Uint128::new(amount2), denom.clone())],
                    };
                    response = response.add_submessage(SubMsg {
                        id: 3,
                        payload: to_json_binary(&Payload {
                            id: 0,
                            action: "SEND".to_string(),
                        })?,
                        msg: msg_send.into(),
                        gas_limit: None,
                        reply_on: ReplyOn::Always,
                    });
                }
                ExecuteMessage::Burn(amount, denom) => {
                    let msg_send = BankMsg::Burn {
                        amount: vec![Coin::new(Uint128::new(amount), denom.clone())],
                    };
                    response = response.add_submessage(SubMsg {
                        id: 4,
                        payload: to_json_binary(&Payload {
                            id: 0,
                            action: "BURN".to_string(),
                        })?,
                        msg: msg_send.into(),
                        gas_limit: None,
                        reply_on: ReplyOn::Always,
                    });
                }
                ExecuteMessage::BurnNoPayload(amount, denom) => {
                    let msg_send = BankMsg::Burn {
                        amount: vec![Coin::new(Uint128::new(amount), denom.clone())],
                    };
                    response = response.add_submessage(SubMsg {
                        id: 5,
                        payload: Binary::default(),
                        msg: msg_send.into(),
                        gas_limit: None,
                        reply_on: ReplyOn::Always,
                    });
                }
                ExecuteMessage::Nop => {}
            }
            Ok(response)
        }

        pub fn query(_deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
            Ok(Binary::default())
        }

        pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
            #[allow(deprecated)]
            let Reply { id, payload, .. } = msg;
            let payload = if let Ok(mut payload) = from_json::<Payload>(payload.clone()) {
                payload.id = id;
                payload
            } else {
                Payload {
                    id,
                    action: "EMPTY".to_string(),
                }
            };
            Ok(Response::new().set_data(to_json_binary(&payload)?))
        }
    }
}
