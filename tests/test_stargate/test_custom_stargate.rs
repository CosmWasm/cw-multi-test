use crate::test_stargate::test_contracts::{MsgCreateDenom, MsgCreateDenomResponse};
use cosmwasm_std::{
    from_json, Addr, AnyMsg, Api, Binary, BlockInfo, CustomMsg, CustomQuery, Empty, MsgResponse,
    Storage, SubMsgResponse,
};
use cw_multi_test::error::AnyResult;
use cw_multi_test::{
    no_init, AppBuilder, AppResponse, Contract, ContractWrapper, CosmosRouter, Executor, IntoAddr,
    Stargate,
};
use prost::Message;
use serde::de::DeserializeOwned;

struct StargateKeeper;

impl Stargate for StargateKeeper {
    fn execute_any<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _sender: Addr,
        msg: AnyMsg,
    ) -> AnyResult<AppResponse>
    where
        ExecC: CustomMsg + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        let message = MsgCreateDenom::decode(msg.value.as_slice())?;
        let msg_create_denom_response = MsgCreateDenomResponse {
            new_token_denom: format!("{}-token-denom", message.subdenom),
        };
        let msg_response = MsgResponse {
            type_url: MsgCreateDenomResponse::TYPE_URL.to_string(),
            value: Binary::from(msg_create_denom_response.encode_to_vec()),
        };
        #[allow(deprecated)]
        let sub_response = SubMsgResponse {
            events: vec![],
            data: None,
            msg_responses: vec![msg_response],
        };
        Ok(sub_response.into())
    }
}

fn the_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new_with_empty(
            super::test_contracts::stargater::execute,
            super::test_contracts::stargater::instantiate,
            super::test_contracts::stargater::query,
        )
        .with_reply(super::test_contracts::stargater::reply),
    )
}

#[test]
fn execute_should_work() {
    // Build the chain simulator with custom stargate keeper.
    let mut app = AppBuilder::default()
        .with_stargate(StargateKeeper)
        .build(no_init);

    let code_id = app.store_code(the_contract());

    let sender = "sender".into_addr();

    let contract_addr = app
        .instantiate_contract(
            code_id,
            sender.clone(),
            &Empty {},
            &[],
            "the-contract",
            None,
        )
        .unwrap();

    // Execute empty message to trigger processing Any message.
    let app_response = app
        .execute_contract(sender, contract_addr.clone(), &Empty {}, &[])
        .unwrap();

    let response = from_json::<String>(app_response.data.unwrap()).unwrap();
    assert_eq!("pao-token-denom", response);
}
