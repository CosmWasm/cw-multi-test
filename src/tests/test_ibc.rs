use crate::test_helpers::{stargate, stargate::ExecMsg};
use crate::{App, AppBuilder, Executor, IbcAcceptingModule, Module};
use cosmwasm_std::testing::MockStorage;
use cosmwasm_std::{Addr, Empty, IbcMsg, IbcQuery};

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

#[test]
fn ibc_accepting_module_works() {
    let ibc_accepting_module = IbcAcceptingModule {};
    let app = App::default();
    let mut storage = MockStorage::default();
    assert!(ibc_accepting_module
        .execute(
            app.api(),
            &mut storage,
            app.router(),
            &app.block_info(),
            Addr::unchecked("sender"),
            IbcMsg::CloseChannel {
                channel_id: "my-channel".to_string()
            }
        )
        .is_ok());
    assert!(ibc_accepting_module
        .query(
            app.api(),
            &storage,
            &(*app.wrap()),
            &app.block_info(),
            IbcQuery::ListChannels { port_id: None }
        )
        .is_ok());
    assert!(ibc_accepting_module
        .sudo(
            app.api(),
            &mut storage,
            app.router(),
            &app.block_info(),
            Empty {}
        )
        .is_ok());
}
