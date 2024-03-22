use crate::test_contracts::counter;
use crate::test_contracts::counter::{CounterQueryMsg, CounterResponseMsg};
use cosmwasm_std::{to_json_binary, Empty, WasmMsg};
use cw_multi_test::{App, Executor};

#[test]
fn read_write_contract_storage_should_work() {
    // prepare the blockchain
    let mut app = App::default();

    // store the contract code
    let creator_addr = app.api().addr_make("creator");
    let code_id = app.store_code_with_creator(creator_addr, counter::contract());
    assert_eq!(1, code_id);

    // instantiate a new contract
    let owner_addr = app.api().addr_make("owner");
    let contract_addr = app
        .instantiate_contract(code_id, owner_addr.clone(), &Empty {}, &[], "counter", None)
        .unwrap();
    assert!(contract_addr.as_str().starts_with("cosmwasm1"));

    // `counter` contract should return value 1 after instantiation
    let query_res: CounterResponseMsg = app
        .wrap()
        .query_wasm_smart(&contract_addr, &CounterQueryMsg::Counter {})
        .unwrap();
    assert_eq!(1, query_res.value);

    // execute `counter` contract - this increments a counter with one
    let execute_msg = WasmMsg::Execute {
        contract_addr: contract_addr.clone().into(),
        msg: to_json_binary(&Empty {}).unwrap(),
        funds: vec![],
    };
    app.execute_contract(owner_addr, contract_addr.clone(), &execute_msg, &[])
        .unwrap();

    // now the `counter` contract should return value 2
    let query_res: CounterResponseMsg = app
        .wrap()
        .query_wasm_smart(&contract_addr, &CounterQueryMsg::Counter {})
        .unwrap();
    assert_eq!(2, query_res.value);

    // change the contract storage, set a value to 100
    // TODO

    // now the `counter` contract should return value 100
    let query_res: CounterResponseMsg = app
        .wrap()
        .query_wasm_smart(&contract_addr, &CounterQueryMsg::Counter {})
        .unwrap();
    assert_eq!(2, query_res.value);
    // TODO assert_eq!(100, query_res.value);
}
