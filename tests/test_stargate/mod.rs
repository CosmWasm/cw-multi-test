#![cfg(feature = "cosmwasm_2_0")]

mod test_custom_stargate;

mod test_contracts {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, PartialEq, Eq, ::prost::Message, Serialize, Deserialize, JsonSchema)]
    pub struct MsgCreateDenom {
        #[prost(string, tag = "1")]
        pub sender: String,
        /// subdenom can be up to 44 "alphanumeric" characters long.
        #[prost(string, tag = "2")]
        pub subdenom: String,
    }

    impl MsgCreateDenom {
        pub const TYPE_URL: &'static str = "multitest.stargate.MsgCreateDenom";
    }

    #[derive(Clone, PartialEq, Eq, ::prost::Message, Serialize, Deserialize, JsonSchema)]
    pub struct MsgCreateDenomResponse {
        #[prost(string, tag = "1")]
        pub new_token_denom: String,
    }

    impl MsgCreateDenomResponse {
        pub const TYPE_URL: &'static str = "multitest.stargate.MsgCreateDenomResponse";
    }

    /// Example smart contract for testing stargate messages.
    pub mod stargater {
        use super::*;
        use ::prost::Message;
        use cosmwasm_std::{
            to_json_binary, AnyMsg, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo,
            Reply, Response, StdResult, SubMsg, SubMsgResponse, SubMsgResult,
        };

        /// Entry-point for instantiating the contract.
        pub fn instantiate(
            _deps: DepsMut,
            _env: Env,
            _info: MessageInfo,
            _msg: Empty,
        ) -> StdResult<Response> {
            Ok(Response::default())
        }

        /// Entry-point for executing contract's messages.
        pub fn execute(
            _deps: DepsMut,
            _env: Env,
            info: MessageInfo,
            _msg: Empty,
        ) -> StdResult<Response> {
            let msg_create_denom = MsgCreateDenom {
                sender: info.sender.into(),
                subdenom: "pao".into(),
            };
            let msg = CosmosMsg::Any(AnyMsg {
                type_url: MsgCreateDenom::TYPE_URL.to_string(),
                value: Binary::from(msg_create_denom.encode_to_vec()),
            });
            Ok(Response::new().add_submessage(SubMsg::reply_always(msg, 1)))
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
                if id == 1
                    && msg_responses.len() == 1
                    && msg_responses[0].type_url == MsgCreateDenomResponse::TYPE_URL
                {
                    if let Ok(response) =
                        MsgCreateDenomResponse::decode(msg_responses[0].value.as_slice())
                    {
                        return Ok(
                            Response::new().set_data(to_json_binary(&response.new_token_denom)?)
                        );
                    }
                }
            }
            Ok(Response::default())
        }
    }
}
