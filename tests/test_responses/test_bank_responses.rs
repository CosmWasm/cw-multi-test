use super::*;
use crate::test_responses::test_contracts::responder::ResponderResponse;
use cosmwasm_std::{from_json, Addr, Binary, Coin, Empty, Uint128};
use cw_multi_test::{App, Contract, ContractWrapper, Executor, IntoAddr};

const DENOM: &str = "pao";

pub fn payloader_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new_with_empty(
            test_contracts::responder::execute,
            test_contracts::responder::instantiate,
            test_contracts::responder::query,
        )
        .with_reply(test_contracts::responder::reply),
    )
}

fn coins(amount: u128) -> Vec<Coin> {
    vec![Coin {
        denom: DENOM.to_string(),
        amount: Uint128::new(amount),
    }]
}

fn assert_balance(app: &App, amount: u128, addr: &Addr) {
    #[allow(deprecated)]
    let coins = app.wrap().query_all_balances(addr).unwrap();
    assert_eq!(1, coins.len());
    assert_eq!(amount, coins[0].amount.u128());
    assert_eq!(DENOM, coins[0].denom);
}

#[test]
fn submessage_responses_from_bank_should_work() {
    //---------------------------------------------------------------------------------------------
    // Chain initialization
    //---------------------------------------------------------------------------------------------

    // Prepare addresses for Alice and Bob.
    let alice_addr = "alice".into_addr();
    let bob_addr = "bob".into_addr();

    // Initialize the chain with initial balances for Alice and Bob.
    let mut app = App::new(|router, _api, storage| {
        router
            .bank
            .init_balance(storage, &alice_addr, coins(1000))
            .unwrap();
        router
            .bank
            .init_balance(storage, &bob_addr, coins(10))
            .unwrap();
    });

    // Check the balance for Alice.
    assert_balance(&app, 1000, &alice_addr);

    // Check the balance for Bob.
    assert_balance(&app, 10, &bob_addr);

    // Alice stores the code of the responder contract on chain.
    let code_id = app.store_code_with_creator(alice_addr.clone(), payloader_contract());

    // Alice instantiates the responder contract, transferring some coins.
    let contract_addr = app
        .instantiate_contract(
            code_id,
            alice_addr.clone(),
            &Empty {},
            &coins(900),
            "responder",
            None,
        )
        .unwrap();

    // Alice should now have only 100 coins, because 900 coins were sent to the instantiated contract.
    assert_balance(&app, 100, &alice_addr);

    // The contract should have 900 coins.
    assert_balance(&app, 900, &contract_addr);

    //---------------------------------------------------------------------------------------------
    // Alice sends 100 coins to Bob using the `responder` contract.
    // Responder contract utilizes BankMsg::Send submessage for this task.
    // The result from processing BankMsg::Send message by the chain is sent back to the contract
    // utilizing the reply entry-point. The msg_responses field sent from the chain
    // if transferred to the caller to verify if processing the submessage returns proper values.
    //---------------------------------------------------------------------------------------------

    let msg = test_contracts::responder::ExecuteMessage::BankSend(
        bob_addr.to_string(),
        100,
        DENOM.to_string(),
    );
    let app_response = app
        .execute_contract(alice_addr.clone(), contract_addr.clone(), &msg, &[])
        .unwrap();
    let responder_response = from_json::<ResponderResponse>(app_response.data.unwrap()).unwrap();

    // The identifier of the reply message should be 1.
    assert_eq!(1, responder_response.id.unwrap());
    // BankMsg::Send should respond with single response.
    assert_eq!(1, responder_response.msg_responses.len());
    // The type of the response should be specific for the BankMsg::Send message
    assert_eq!(
        "/cosmos.bank.v1beta1.MsgSendResponse",
        responder_response.msg_responses[0].type_url
    );
    // The value should be empty.
    assert_eq!(Binary::default(), responder_response.msg_responses[0].value);
    // Bob should have now 100 coins more, because Alice sent him 100.
    assert_balance(&app, 110, &bob_addr);
    // No changes for Alice.
    assert_balance(&app, 100, &alice_addr);
    // Now the contract should have 800 coins.
    assert_balance(&app, 800, &contract_addr);
}
