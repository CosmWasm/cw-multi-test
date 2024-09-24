#![cfg(feature = "stargate")]

use crate::test_helpers::ibc;
use crate::{no_init, App, AppBuilder, Executor, IbcAcceptingModule};
use cosmwasm_std::Empty;

#[test]
fn default_ibc() {
    let mut app = App::default();

    let creator_addr = app.api().addr_make("creator");
    let code = app.store_code_with_creator(creator_addr, ibc::contract());

    let owner_addr = app.api().addr_make("owner");
    let contract = app
        .instantiate_contract(code, owner_addr.clone(), &Empty {}, &[], "ibanera", None)
        .unwrap();

    app.execute_contract(owner_addr, contract, &Empty {}, &[])
        .unwrap_err();
}

#[test]
fn accepting_ibc() {
    let mut app = AppBuilder::default()
        .with_ibc(IbcAcceptingModule::new())
        .build(no_init);

    let creator_addr = app.api().addr_make("creator");
    let code = app.store_code_with_creator(creator_addr, ibc::contract());

    let owner_addr = app.api().addr_make("owner");
    let contract = app
        .instantiate_contract(code, owner_addr.clone(), &Empty {}, &[], "ibanera", None)
        .unwrap();

    app.execute_contract(owner_addr, contract, &Empty {}, &[])
        .unwrap();
}
