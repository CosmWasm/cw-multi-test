use super::*;
use crate::test_responses::test_contracts::responder::{
    ResponderInstantiateMessage, ResponderResponse,
};
use cosmwasm_std::{from_json, Empty};
use cw_multi_test::{App, Contract, ContractWrapper, Executor, IntoAddr};

pub fn responder_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new_with_empty(
            test_contracts::responder::execute,
            test_contracts::responder::instantiate,
            test_contracts::responder::query,
        )
        .with_reply_empty(test_contracts::responder::reply),
    )
}

#[test]
fn submessage_responses_from_wasm_execute_should_work() {
    //---------------------------------------------------------------------------------------------
    // Chain initialization.
    //---------------------------------------------------------------------------------------------

    // Prepare addresses for Alice and Bob.
    let alice_addr = "alice".into_addr();

    // Initialize the chain with initial balances for Alice.
    let mut app = App::default();

    // Alice stores the code of the responder contract on chain.
    let code_id = app.store_code_with_creator(alice_addr.clone(), responder_contract());

    // Alice instantiates first responder contract.
    let contract_addr_1 = app
        .instantiate_contract(
            code_id,
            alice_addr.clone(),
            &ResponderInstantiateMessage::None,
            &[],
            "responder-1",
            None,
        )
        .unwrap();

    // Alice instantiates the second responder contract.
    let contract_addr_2 = app
        .instantiate_contract(
            code_id,
            alice_addr.clone(),
            &ResponderInstantiateMessage::None,
            &[],
            "responder-2",
            None,
        )
        .unwrap();

    //---------------------------------------------------------------------------------------------
    // Alice executes Wasm::Execute as a submessage on the contract_addr_1.
    // The result from processing Wasm::Execute message by the chain is sent back to the
    // contract_addr_1, utilizing the reply entry-point. The msg_responses field sent from
    // the chain if transferred to the caller to verify if processing the submessage
    // returns proper values.
    //---------------------------------------------------------------------------------------------

    let msg = test_contracts::responder::ResponderExecuteMessage::WasmMsgExecuteAdd(
        contract_addr_2.to_string(),
        263,
        87,
    );
    let app_response = app
        .execute_contract(alice_addr.clone(), contract_addr_1, &msg, &[])
        .unwrap();

    let responder_response = from_json::<ResponderResponse>(app_response.data.unwrap()).unwrap();

    // The identifier of the reply message should be 3.
    assert_eq!(3, responder_response.id.unwrap());
    // There should be a single submessage in the reply response.
    assert_eq!(1, responder_response.msg_responses.len());
    assert_eq!(
        "/cosmwasm.wasm.v1.MsgExecuteContractResponse",
        responder_response.msg_responses[0].type_url
    );
    assert_eq!(
        &[10, 3, 51, 53, 48],
        responder_response.msg_responses[0].value.as_slice()
    );
}
