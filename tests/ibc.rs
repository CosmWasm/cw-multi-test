use cosmwasm_std::{Addr, Empty};
use cw_multi_test::testing_helpers::{stargate, stargate::ExecMsg};
use cw_multi_test::{App, AppBuilder, Executor, IbcAcceptingModule};

#[test]
fn default_ibc() {
    let mut app = App::default();
    let code = app.store_code(stargate::contract());
    let contract = app
        .instantiate_contract(
            code,
            Addr::unchecked("owner"),
            &Empty {},
            &[],
            "contract",
            None,
        )
        .unwrap();

    app.execute_contract(Addr::unchecked("owner"), contract, &ExecMsg::Ibc {}, &[])
        .unwrap_err();
}

#[test]
fn substituting_ibc() {
    let mut app = AppBuilder::new()
        .with_ibc(IbcAcceptingModule)
        .build(|_, _, _| ());
    let code = app.store_code(stargate::contract());
    let contract = app
        .instantiate_contract(
            code,
            Addr::unchecked("owner"),
            &Empty {},
            &[],
            "contract",
            None,
        )
        .unwrap();

    app.execute_contract(Addr::unchecked("owner"), contract, &ExecMsg::Ibc {}, &[])
        .unwrap();
}
