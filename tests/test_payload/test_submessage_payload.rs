use super::*;
use crate::test_payload::test_contracts::payloader::Payload;
use cosmwasm_std::{from_json, Addr, Coin, Empty, Uint256};
use cw_multi_test::{App, Contract, ContractWrapper, Executor, IntoAddr};

const DENOM: &str = "pao";

pub fn payloader_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new_with_empty(
            test_contracts::payloader::execute,
            test_contracts::payloader::instantiate,
            test_contracts::payloader::query,
        )
        .with_reply(test_contracts::payloader::reply),
    )
}

fn coins(amount: u128) -> Vec<Coin> {
    vec![Coin {
        denom: DENOM.to_string(),
        amount: Uint256::new(amount),
    }]
}

fn assert_balance(app: &App, amount: u128, addr: &Addr) {
    #[allow(deprecated)]
    let coin = app.wrap().query_balance(addr, DENOM).unwrap();
    assert_eq!(Uint256::new(amount), coin.amount);
}

#[test]
fn submessage_payload_should_work() {
    // Prepare addresses for Alice, Bob and Cecil accounts.
    let alice_addr = "alice".into_addr();
    let bob_addr = "bob".into_addr();
    let cecil_addr = "cecil".into_addr();

    // Initialize the chain with initial balances for Alice, Bob and Cecil.
    let mut app = App::new(|router, _api, storage| {
        router
            .bank
            .init_balance(storage, &alice_addr, coins(100))
            .unwrap();
        router
            .bank
            .init_balance(storage, &bob_addr, coins(200))
            .unwrap();
        router
            .bank
            .init_balance(storage, &cecil_addr, coins(300))
            .unwrap();
    });

    // Check the balance for Alice.
    assert_balance(&app, 100, &alice_addr);

    // Check the balance for Bob.
    assert_balance(&app, 200, &bob_addr);

    // Check the balance for Cecil.
    assert_balance(&app, 300, &cecil_addr);

    // Alice stores the code of payloader contract on chain.
    let code_id = app.store_code_with_creator(alice_addr.clone(), payloader_contract());

    // Alice instantiates the contract.
    let contract_addr = app
        .instantiate_contract(
            code_id,
            alice_addr.clone(),
            &Empty {},
            &coins(90),
            "payloader",
            None,
        )
        .unwrap();

    // Now, Alice should have only 10pao, because 90pao was sent to the instantiated contract.
    assert_balance(&app, 10, &alice_addr);

    // Now the contract should have 90pao.
    assert_balance(&app, 90, &contract_addr);

    //---------------------------------------------------------------------------------------------
    // Alice sends 10pao to Bob using payloader contract.
    //
    // 1. Alice executes ExecuteMessage::Send message variant on payloader contract.
    // 2. Payloader contract returns single submessage that is the Bank::Send variant
    //    with the address of Bob.
    // 3. Chain processes the bank message (sends 10pao to Bob) and returns to payloader contract
    //    with the response executing the reply entrypoint.
    // 4. The response from processing the execute and reply is the data returned from reply.
    //    Because there was only one submessage, the result is the data from reply processed
    //    for this single submessage.
    //---------------------------------------------------------------------------------------------

    // Alice sends 10pao to Bob using payloader contract.
    let msg = test_contracts::payloader::ExecuteMessage::Send(
        bob_addr.to_string(),
        10,
        DENOM.to_string(),
    );
    let response = app
        .execute_contract(alice_addr.clone(), contract_addr.clone(), &msg, &[])
        .unwrap();
    let payload = from_json::<Payload>(response.data.unwrap()).unwrap();
    assert_eq!(1, payload.id);
    assert_eq!("SEND", payload.action);

    // Now, Bob should have 10pao more, because Alice sent him 10pao.
    assert_balance(&app, 210, &bob_addr);

    // No changes for Cecil.
    assert_balance(&app, 300, &cecil_addr);

    // No changes for Alice.
    assert_balance(&app, 10, &alice_addr);

    // Now the contract should have 80pao.
    assert_balance(&app, 80, &contract_addr);

    //---------------------------------------------------------------------------------------------
    // Alice sends 1pao to Bob and 2pao to Cecil using payloader contract.
    //
    // 1. Alice executes ExecuteMessage::SendMulti message variant on payloader contract.
    // 2. Payloader contract returns two submessages that are the Bank::Send variants
    //    one with the address of Bob and second with the address of Cecil.
    // 3. Chain processes both bank messages (sends 1pao to Bob nad 2pao to Cecil)
    //    and returns to payloader contract with the responses executing the reply
    //    entrypoint twice.
    // 4. The response from processing the execute and two replies is the data returned
    //    from processing the LAST REPLY.
    //---------------------------------------------------------------------------------------------

    // Alice sends 1pao to Bob and 2pao to Cecil using payloader contract.
    let msg = test_contracts::payloader::ExecuteMessage::SendMulti(
        bob_addr.to_string(),
        1,
        cecil_addr.to_string(),
        2,
        DENOM.to_string(),
    );
    let response = app
        .execute_contract(alice_addr.clone(), contract_addr.clone(), &msg, &[])
        .unwrap();
    let payload = from_json::<Payload>(response.data.unwrap()).unwrap();
    assert_eq!(3, payload.id);
    assert_eq!("SEND", payload.action);

    // Now, Bob should have 1pao more, because Alice sent him 10pao.
    assert_balance(&app, 211, &bob_addr);

    // Now, Cecil should have 2pao more, because Alice sent him 10pao.
    assert_balance(&app, 302, &cecil_addr);

    // No changes for Alice.
    assert_balance(&app, 10, &alice_addr);

    // Now the contract should have 77pao.
    assert_balance(&app, 77, &contract_addr);

    //---------------------------------------------------------------------------------------------
    // Alice burns 10pao in her contract.
    //
    // 1. Alice executes ExecuteMessage::Burn message variant on payloader contract.
    // 2. Payloader contract returns two submessage that is the Bank::Burn variant.
    // 3. Chain processes both bank message (burns 10pao for the contract)
    //    and returns to payloader contract with the response executing the reply entrypoint.
    // 4. The response is from reply entrypoint.
    //---------------------------------------------------------------------------------------------

    let msg = test_contracts::payloader::ExecuteMessage::Burn(10, DENOM.to_string());
    let response = app
        .execute_contract(alice_addr.clone(), contract_addr.clone(), &msg, &[])
        .unwrap();
    let payload = from_json::<Payload>(response.data.unwrap()).unwrap();
    assert_eq!(4, payload.id);
    assert_eq!("BURN", payload.action);

    // No changes for Bob.
    assert_balance(&app, 211, &bob_addr);

    // No changes for Cecil.
    assert_balance(&app, 302, &cecil_addr);

    // No changes for Alice.
    assert_balance(&app, 10, &alice_addr);

    // Now the contract should have 67pao.
    assert_balance(&app, 67, &contract_addr);

    //---------------------------------------------------------------------------------------------
    // Alice burns 10pao in her contract.
    // This time without filling the payload field in the submessage.
    //---------------------------------------------------------------------------------------------

    let msg = test_contracts::payloader::ExecuteMessage::BurnNoPayload(10, DENOM.to_string());
    let response = app
        .execute_contract(alice_addr.clone(), contract_addr.clone(), &msg, &[])
        .unwrap();
    let payload = from_json::<Payload>(response.data.unwrap()).unwrap();
    assert_eq!(5, payload.id);
    assert_eq!("EMPTY", payload.action);

    // No changes for Bob.
    assert_balance(&app, 211, &bob_addr);

    // No changes for Cecil.
    assert_balance(&app, 302, &cecil_addr);

    // No changes for Alice.
    assert_balance(&app, 10, &alice_addr);

    // Now the contract should have 57pao.
    assert_balance(&app, 57, &contract_addr);

    //---------------------------------------------------------------------------------------------
    // Alice executes a variant without reply.
    // The response is from the execute entrypoint.
    //---------------------------------------------------------------------------------------------

    let msg = test_contracts::payloader::ExecuteMessage::Nop;
    let response = app
        .execute_contract(alice_addr.clone(), contract_addr.clone(), &msg, &[])
        .unwrap();
    let payload = from_json::<Payload>(response.data.unwrap()).unwrap();
    assert_eq!(0, payload.id);
    assert_eq!("EXECUTE", payload.action);

    // No changes for Bob.
    assert_balance(&app, 211, &bob_addr);

    // No changes for Cecil.
    assert_balance(&app, 302, &cecil_addr);

    // No changes for Alice.
    assert_balance(&app, 10, &alice_addr);

    // Now changes for the contract.
    assert_balance(&app, 57, &contract_addr);
}
