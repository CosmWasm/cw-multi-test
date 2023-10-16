use crate::test_app_builder::{contracts, CounterQueryMsg, CounterResponseMsg};
use cosmwasm_std::{to_binary, Addr, Empty, Order, Record, Storage, WasmMsg};
use cw_multi_test::{AppBuilder, Executor};
use std::collections::BTreeMap;
use std::iter;

#[derive(Default)]
struct MyStorage(BTreeMap<Vec<u8>, Vec<u8>>);

// Minimal implementation of custom storage.
impl Storage for MyStorage {
    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.0.get::<Vec<u8>>(&key.into()).cloned()
    }

    fn range<'a>(
        &'a self,
        _start: Option<&[u8]>,
        _end: Option<&[u8]>,
        _order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        Box::new(iter::empty())
    }

    fn set(&mut self, key: &[u8], value: &[u8]) {
        self.0.insert(key.into(), value.into());
    }

    fn remove(&mut self, key: &[u8]) {
        self.0.remove(key);
    }
}

#[test]
fn building_app_with_custom_storage_should_work() {
    // prepare additional test input data
    let owner = Addr::unchecked("owner");
    let msg = to_binary(&Empty {}).unwrap();
    let admin = None;
    let funds = vec![];
    let label = "my-counter";

    // build the application with custom storage
    let app_builder = AppBuilder::default();
    let mut app = app_builder
        .with_storage(MyStorage::default())
        .build(|_, _, _| {});

    // store a contract code
    let code_id = app.store_code(contracts::counter::contract());

    // instantiate contract, this initializes a counter with value 1
    let contract_addr = app
        .instantiate_contract(
            code_id,
            owner.clone(),
            &WasmMsg::Instantiate {
                admin: admin.clone(),
                code_id,
                msg: msg.clone(),
                funds: funds.clone(),
                label: label.into(),
            },
            &funds,
            label,
            admin,
        )
        .unwrap();

    // execute contract, this increments a counter
    app.execute_contract(
        owner,
        contract_addr.clone(),
        &WasmMsg::Execute {
            contract_addr: contract_addr.clone().into(),
            msg,
            funds,
        },
        &[],
    )
    .unwrap();

    // query contract for current counter value
    let response: CounterResponseMsg = app
        .wrap()
        .query_wasm_smart(&contract_addr, &CounterQueryMsg::Counter {})
        .unwrap();

    // counter should be 2
    assert_eq!(2, response.value);
}
