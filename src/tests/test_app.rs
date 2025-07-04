use crate::custom_handler::CachingCustomHandler;
use crate::featured::staking::{Distribution, Staking};
use crate::test_helpers::echo::EXECUTE_REPLY_BASE_ID;
use crate::test_helpers::{caller, echo, error, hackatom, payout, reflect, CustomHelperMsg};
use crate::transactions::{transactional, StorageTransaction};
use crate::wasm::ContractData;
use crate::{
    custom_app, next_block, no_init, App, AppResponse, Bank, CosmosRouter, Executor, Module,
    Router, Wasm, WasmSudo,
};
use crate::{AppBuilder, IntoAddr};
use cosmwasm_std::testing::{mock_env, MockQuerier};
use cosmwasm_std::{
    coin, coins, from_json, to_json_binary, Addr, Api, Attribute, BalanceResponse, BankMsg,
    BankQuery, Binary, BlockInfo, Coin, CosmosMsg, CustomMsg, CustomQuery, Empty, Event, Querier,
    Reply, StdResult, Storage, SubMsg, WasmMsg,
};
use cw_storage_plus::Item;
use cw_utils::parse_instantiate_response_data;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Utility function that returns all balances for specified address.
fn get_balance<BankT, ApiT, StorageT, CustomT, WasmT>(
    app: &App<BankT, ApiT, StorageT, CustomT, WasmT>,
    address: &Addr,
    denom: &str,
) -> Coin
where
    CustomT::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    ApiT: Api,
    StorageT: Storage,
    CustomT: Module,
{
    app.wrap().query_balance(address, denom).unwrap()
}

fn query_router<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>(
    router: &Router<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>,
    api: &dyn Api,
    storage: &dyn Storage,
    rcpt: &Addr,
    denom: &str,
) -> Coin
where
    CustomT::ExecT: CustomMsg,
    CustomT::QueryT: CustomQuery + DeserializeOwned,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    CustomT: Module,
    StakingT: Staking,
    DistrT: Distribution,
{
    let query = BankQuery::Balance {
        address: rcpt.into(),
        denom: denom.to_string(),
    };
    let block = mock_env().block;
    let querier: MockQuerier<CustomT::QueryT> = MockQuerier::new(&[]);
    let res = router
        .bank
        .query(api, storage, &querier, &block, query)
        .unwrap();
    let val: BalanceResponse = from_json(res).unwrap();
    val.amount
}

fn query_app<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT>(
    app: &App<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT>,
    rcpt: &Addr,
    denom: &str,
) -> Coin
where
    CustomT::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    ApiT: Api,
    StorageT: Storage,
    CustomT: Module,
    StakingT: Staking,
    DistrT: Distribution,
{
    let query = BankQuery::Balance {
        address: rcpt.into(),
        denom: denom.to_string(),
    }
    .into();
    let val: BalanceResponse = app.wrap().query(&query).unwrap();
    val.amount
}

/// Utility function for generating user addresses.
fn addr_make(addr: &str) -> Addr {
    addr.into_addr()
}

#[test]
fn update_block() {
    let mut app = App::default();
    let BlockInfo { time, height, .. } = app.block_info();
    app.update_block(next_block);
    assert_eq!(time.plus_seconds(5), app.block_info().time);
    assert_eq!(height + 1, app.block_info().height);
}

#[test]
fn multi_level_bank_cache() {
    // prepare user addresses
    let owner_addr = addr_make("owner");
    let recipient_addr = addr_make("recipient");

    // set personal balance
    let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner_addr, init_funds)
            .unwrap();
    });

    // cache 1 - send some tokens
    let mut cache = StorageTransaction::new(app.storage());
    let msg = BankMsg::Send {
        to_address: recipient_addr.clone().into(),
        amount: coins(25, "eth"),
    };
    app.router()
        .execute(
            app.api(),
            &mut cache,
            &app.block_info(),
            owner_addr.clone(),
            msg.into(),
        )
        .unwrap();

    // shows up in cache
    assert_eq!(
        coin(25, "eth"),
        query_router(app.router(), app.api(), &cache, &recipient_addr, "eth")
    );
    assert_eq!(coin(0, "eth"), query_app(&app, &recipient_addr, "eth"));

    // now, second level cache
    transactional(&mut cache, |cache2, read| {
        let msg = BankMsg::Send {
            to_address: recipient_addr.clone().into(),
            amount: coins(12, "eth"),
        };
        app.router()
            .execute(app.api(), cache2, &app.block_info(), owner_addr, msg.into())
            .unwrap();

        // shows up in 2nd cache
        assert_eq!(
            coin(25, "eth"),
            query_router(app.router(), app.api(), read, &recipient_addr, "eth")
        );
        assert_eq!(
            coin(37, "eth"),
            query_router(app.router(), app.api(), cache2, &recipient_addr, "eth")
        );
        Ok(())
    })
    .unwrap();

    // apply first to router
    cache.prepare().commit(app.storage_mut());

    assert_eq!(coin(37, "eth"), query_app(&app, &recipient_addr, "eth"));
}

#[test]
#[cfg(feature = "cosmwasm_1_2")]
fn duplicate_contract_code() {
    // set up the multi-test application
    let mut app = App::default();

    // store the original contract code
    let code_id = app.store_code(payout::contract());

    // duplicate previously stored contract code
    let dup_code_id = app.duplicate_code(code_id).unwrap();
    assert_ne!(code_id, dup_code_id);

    // query and compare code info of both contract_helpers
    let response = app.wrap().query_wasm_code_info(code_id).unwrap();
    let dup_response = app.wrap().query_wasm_code_info(dup_code_id).unwrap();
    assert_ne!(response.code_id, dup_response.code_id);
    assert_eq!(response.creator, dup_response.creator);
    assert_eq!(response.checksum, dup_response.checksum);
}

#[test]
fn send_tokens() {
    // prepare user addresses
    let owner_addr = addr_make("owner");
    let recipient_addr = addr_make("recipient");

    // set personal balance
    let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
    let rcpt_funds = vec![coin(5, "btc")];
    let mut app = App::new(|router, _, storage| {
        // initialization moved to App construction
        router
            .bank
            .init_balance(storage, &owner_addr, init_funds)
            .unwrap();
        router
            .bank
            .init_balance(storage, &recipient_addr, rcpt_funds)
            .unwrap();
    });

    // send both tokens
    let to_send = vec![coin(30, "eth"), coin(5, "btc")];
    let msg: CosmosMsg = BankMsg::Send {
        to_address: recipient_addr.clone().into(),
        amount: to_send,
    }
    .into();
    app.execute(owner_addr.clone(), msg.clone()).unwrap();
    assert_eq!(coin(15, "btc"), get_balance(&app, &owner_addr, "btc"));
    assert_eq!(coin(70, "eth"), get_balance(&app, &owner_addr, "eth"));
    assert_eq!(coin(10, "btc"), get_balance(&app, &recipient_addr, "btc"));
    assert_eq!(coin(30, "eth"), get_balance(&app, &recipient_addr, "eth"));

    // can send from other account (but funds will be deducted from sender)
    app.execute(recipient_addr.clone(), msg).unwrap();

    // cannot send too much
    let msg = BankMsg::Send {
        to_address: recipient_addr.into(),
        amount: coins(20, "btc"),
    }
    .into();
    app.execute(owner_addr.clone(), msg).unwrap_err();

    assert_eq!(coin(15, "btc"), get_balance(&app, &owner_addr, "btc"));
    assert_eq!(coin(70, "eth"), get_balance(&app, &owner_addr, "eth"));
}

#[test]
fn simple_contract() {
    // prepare user addresses
    let owner_addr = addr_make("owner");

    // set personal balance
    let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner_addr, init_funds)
            .unwrap();
    });

    // set up contract
    let code_id = app.store_code(payout::contract());

    let msg = payout::InstantiateMessage {
        payout: coin(5, "eth"),
    };
    let contract_addr = app
        .instantiate_contract(
            code_id,
            owner_addr.clone(),
            &msg,
            &coins(23, "eth"),
            "Payout",
            None,
        )
        .unwrap();

    let contract_data = app.contract_data(&contract_addr).unwrap();
    assert_eq!(
        contract_data,
        ContractData {
            code_id,
            creator: owner_addr.clone(),
            admin: None,
            label: "Payout".to_owned(),
            created: app.block_info().height
        }
    );

    // sender funds deducted
    assert_eq!(coin(20, "btc"), get_balance(&app, &owner_addr, "btc"));
    assert_eq!(coin(77, "eth"), get_balance(&app, &owner_addr, "eth"));
    // get contract address, has funds
    assert_eq!(coin(23, "eth"), get_balance(&app, &contract_addr, "eth"));

    // create empty account
    let random_addr = app.api().addr_make("random");
    assert_eq!(coin(0, "btc"), get_balance(&app, &random_addr, "btc"));
    assert_eq!(coin(0, "eth"), get_balance(&app, &random_addr, "eth"));

    // do one payout and see money coming in
    let res = app
        .execute_contract(random_addr.clone(), contract_addr.clone(), &Empty {}, &[])
        .unwrap();
    assert_eq!(3, res.events.len());

    // the call to payout does emit this as well as custom attributes
    let payout_exec = &res.events[0];
    assert_eq!(payout_exec.ty.as_str(), "execute");
    assert_eq!(
        payout_exec.attributes,
        [("_contract_address", &contract_addr)]
    );

    // next is a custom wasm event
    let custom_attrs = res.custom_attrs(1);
    assert_eq!(custom_attrs, [("action", "payout")]);

    // then the transfer event
    let expected_transfer = Event::new("transfer")
        .add_attribute("recipient", &random_addr)
        .add_attribute("sender", &contract_addr)
        .add_attribute("amount", "5eth");
    assert_eq!(&expected_transfer, &res.events[2]);

    // random got cash
    assert_eq!(coin(5, "eth"), get_balance(&app, &random_addr, "eth"));
    // contract lost it
    assert_eq!(coin(18, "eth"), get_balance(&app, &contract_addr, "eth"));
}

#[test]
fn reflect_success() {
    // prepare user addresses
    let owner_addr = addr_make("owner");
    let random_addr = addr_make("random");

    // set personal balance
    let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
    let mut app = custom_app::<CustomHelperMsg, Empty, _>(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner_addr, init_funds)
            .unwrap();
    });

    // set up payout contract
    let payout_code_id = app.store_code(payout::contract());

    let msg = payout::InstantiateMessage {
        payout: coin(5, "eth"),
    };
    let payout_addr = app
        .instantiate_contract(
            payout_code_id,
            owner_addr.clone(),
            &msg,
            &coins(23, "eth"),
            "Payout",
            None,
        )
        .unwrap();

    // set up reflect contract
    let reflect_code_id = app.store_code(reflect::contract());

    let reflect_addr = app
        .instantiate_contract(reflect_code_id, owner_addr, &Empty {}, &[], "Reflect", None)
        .unwrap();

    // reflect account is empty
    assert_eq!(coin(0, "btc"), get_balance(&app, &reflect_addr, "btc"));
    assert_eq!(coin(0, "eth"), get_balance(&app, &reflect_addr, "eth"));
    // reflect count is 1
    let query_res: payout::CountResponse = app
        .wrap()
        .query_wasm_smart(&reflect_addr, &reflect::QueryMessage::Count)
        .unwrap();
    assert_eq!(0, query_res.count);

    // reflecting payout message pays reflect contract
    let msg = SubMsg::<Empty>::new(WasmMsg::Execute {
        contract_addr: payout_addr.clone().into(),
        msg: b"{}".into(),
        funds: vec![],
    });
    let msgs = reflect::ExecMessage { sub_msg: vec![msg] };
    let res = app
        .execute_contract(random_addr, reflect_addr.clone(), &msgs, &[])
        .unwrap();

    // ensure the attributes were relayed from the sub-message
    assert_eq!(4, res.events.len(), "{:?}", res.events);

    // reflect only returns standard wasm-execute event
    let ref_exec = &res.events[0];
    assert_eq!(ref_exec.ty.as_str(), "execute");
    assert_eq!(ref_exec.attributes, [("_contract_address", &reflect_addr)]);

    // the call to payout does emit this as well as custom attributes
    let payout_exec = &res.events[1];
    assert_eq!(payout_exec.ty.as_str(), "execute");
    assert_eq!(
        payout_exec.attributes,
        [("_contract_address", &payout_addr)]
    );

    let payout = &res.events[2];
    assert_eq!(payout.ty.as_str(), "wasm");
    assert_eq!(
        payout.attributes,
        [
            ("_contract_address", payout_addr.as_str()),
            ("action", "payout")
        ]
    );

    // final event is the transfer from bank
    let second = &res.events[3];
    assert_eq!(second.ty.as_str(), "transfer");
    assert_eq!(3, second.attributes.len());
    assert_eq!(second.attributes[0], ("recipient", &reflect_addr));
    assert_eq!(second.attributes[1], ("sender", &payout_addr));
    assert_eq!(second.attributes[2], ("amount", "5eth"));

    // ensure transfer was executed with reflect as sender
    assert_eq!(coin(5, "eth"), get_balance(&app, &reflect_addr, "eth"));

    // reflect count updated
    let query_res: payout::CountResponse = app
        .wrap()
        .query_wasm_smart(&reflect_addr, &reflect::QueryMessage::Count)
        .unwrap();
    assert_eq!(1, query_res.count);
}

#[test]
fn reflect_error() {
    // prepare user addresses
    let owner = addr_make("owner");

    // set personal balance
    let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
    let mut app = custom_app::<CustomHelperMsg, Empty, _>(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner, init_funds)
            .unwrap();
    });

    // set up reflect contract
    let reflect_id = app.store_code(reflect::contract());

    let reflect_addr = app
        .instantiate_contract(
            reflect_id,
            owner,
            &Empty {},
            &coins(40, "eth"),
            "Reflect",
            None,
        )
        .unwrap();

    // reflect has 40 eth
    assert_eq!(coin(40, "eth"), get_balance(&app, &reflect_addr, "eth"));
    let random_addr = app.api().addr_make("random");

    // sending 7 eth works
    let msg = SubMsg::<Empty>::new(BankMsg::Send {
        to_address: random_addr.clone().into(),
        amount: coins(7, "eth"),
    });
    let msgs = reflect::ExecMessage { sub_msg: vec![msg] };
    let res = app
        .execute_contract(random_addr.clone(), reflect_addr.clone(), &msgs, &[])
        .unwrap();
    // no wasm events as no attributes
    assert_eq!(2, res.events.len());
    // standard wasm-execute event
    let exec = &res.events[0];
    assert_eq!(exec.ty.as_str(), "execute");
    assert_eq!(exec.attributes, [("_contract_address", &reflect_addr)]);
    // only transfer event from bank
    let transfer = &res.events[1];
    assert_eq!(transfer.ty.as_str(), "transfer");

    // ensure random got paid
    assert_eq!(coin(7, "eth"), get_balance(&app, &random_addr, "eth"));

    // reflect count should be updated to 1
    let query_res: payout::CountResponse = app
        .wrap()
        .query_wasm_smart(&reflect_addr, &reflect::QueryMessage::Count)
        .unwrap();
    assert_eq!(1, query_res.count);

    // sending 8 eth, then 3 btc should fail both
    let msg = SubMsg::<Empty>::new(BankMsg::Send {
        to_address: random_addr.clone().into(),
        amount: coins(8, "eth"),
    });
    let msg2 = SubMsg::<Empty>::new(BankMsg::Send {
        to_address: random_addr.clone().into(),
        amount: coins(3, "btc"),
    });
    let msgs = reflect::ExecMessage {
        sub_msg: vec![msg, msg2],
    };
    let err = app
        .execute_contract(random_addr.clone(), reflect_addr.clone(), &msgs, &[])
        .unwrap_err();

    let err_str = err.to_string();
    assert!(
        err_str.starts_with("kind: Other, error: Error executing WasmMsg")
            && err_str.contains("Cannot Sub with given operands")
    );

    // first one should have been rolled-back on error (no second payment)
    assert_eq!(coin(7, "eth"), get_balance(&app, &random_addr, "eth"));

    // failure should not update reflect count
    let query_res: payout::CountResponse = app
        .wrap()
        .query_wasm_smart(&reflect_addr, &reflect::QueryMessage::Count)
        .unwrap();
    assert_eq!(1, query_res.count);
}

#[test]
fn sudo_works() {
    // prepare user addresses
    let owner_addr = addr_make("owner");

    // set personal balance
    let init_funds = vec![coin(100, "eth")];
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner_addr, init_funds)
            .unwrap();
    });

    let payout_id = app.store_code(payout::contract());

    let msg = payout::InstantiateMessage {
        payout: coin(5, "eth"),
    };
    let payout_addr = app
        .instantiate_contract(
            payout_id,
            owner_addr,
            &msg,
            &coins(23, "eth"),
            "Payout",
            None,
        )
        .unwrap();

    // count is 1
    let payout::CountResponse { count } = app
        .wrap()
        .query_wasm_smart(&payout_addr, &payout::QueryMsg::Count {})
        .unwrap();
    assert_eq!(1, count);

    // wasm_sudo call
    let msg = payout::SudoMsg { set_count: 25 };
    app.wasm_sudo(payout_addr.clone(), &msg).unwrap();

    // count is 25
    let payout::CountResponse { count } = app
        .wrap()
        .query_wasm_smart(&payout_addr, &payout::QueryMsg::Count {})
        .unwrap();
    assert_eq!(25, count);

    // we can do the same with sudo call
    let msg = payout::SudoMsg { set_count: 49 };
    let sudo_msg = WasmSudo {
        contract_addr: payout_addr.clone(),
        message: to_json_binary(&msg).unwrap(),
    };
    app.sudo(sudo_msg.into()).unwrap();

    let payout::CountResponse { count } = app
        .wrap()
        .query_wasm_smart(&payout_addr, &payout::QueryMsg::Count {})
        .unwrap();
    assert_eq!(49, count);
}

#[test]
fn reflect_sub_message_reply_works() {
    // prepare user addresses
    let owner = addr_make("owner");
    let random = addr_make("random");

    // set personal balance
    let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
    let mut app = custom_app::<CustomHelperMsg, Empty, _>(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner, init_funds)
            .unwrap();
    });

    // set up reflect contract
    let reflect_id = app.store_code(reflect::contract());

    let reflect_addr = app
        .instantiate_contract(
            reflect_id,
            owner,
            &Empty {},
            &coins(40, "eth"),
            "Reflect",
            None,
        )
        .unwrap();

    // no reply written beforehand
    let query = reflect::QueryMessage::Reply { id: 123 };
    let res: StdResult<Reply> = app.wrap().query_wasm_smart(&reflect_addr, &query);
    res.unwrap_err();

    // reflect sends 7 eth, success
    let msg = SubMsg::<Empty>::reply_always(
        BankMsg::Send {
            to_address: random.clone().into(),
            amount: coins(7, "eth"),
        },
        123,
    );
    let msgs = reflect::ExecMessage { sub_msg: vec![msg] };
    let res = app
        .execute_contract(random.clone(), reflect_addr.clone(), &msgs, &[])
        .unwrap();

    // expected events: execute, transfer, reply, custom wasm (set in reply)
    assert_eq!(4, res.events.len(), "{:?}", res.events);
    res.assert_event(&Event::new("execute").add_attribute("_contract_address", &reflect_addr));
    res.assert_event(&Event::new("transfer").add_attribute("amount", "7eth"));
    res.assert_event(
        &Event::new("reply")
            .add_attribute("_contract_address", reflect_addr.as_str())
            .add_attribute("mode", "handle_success"),
    );
    res.assert_event(&Event::new("wasm-custom").add_attribute("from", "reply"));

    // ensure success was written
    let res: Reply = app.wrap().query_wasm_smart(&reflect_addr, &query).unwrap();
    assert_eq!(res.id, 123);
    // validate the events written in the reply blob...should just be bank transfer
    let reply = res.result.unwrap();
    assert_eq!(1, reply.events.len());
    AppResponse::from(reply).assert_event(&Event::new("transfer").add_attribute("amount", "7eth"));

    // reflect sends 300 btc, failure, but error caught by sub-message (so shows success)
    let msg = SubMsg::<Empty>::reply_always(
        BankMsg::Send {
            to_address: random.clone().into(),
            amount: coins(300, "btc"),
        },
        456,
    );
    let msgs = reflect::ExecMessage { sub_msg: vec![msg] };
    let _res = app
        .execute_contract(random, reflect_addr.clone(), &msgs, &[])
        .unwrap();

    // ensure error was written
    let query = reflect::QueryMessage::Reply { id: 456 };
    let res: Reply = app.wrap().query_wasm_smart(&reflect_addr, &query).unwrap();
    assert_eq!(res.id, 456);
    assert!(res.result.is_err());
}

#[test]
fn send_update_admin_works() {
    // The plan:
    // create a hackatom contract
    // check admin set properly
    // update admin succeeds if admin
    // update admin fails if not (new) admin
    // check admin set properly
    let mut app = App::default();

    let owner = addr_make("owner");
    let owner2 = addr_make("owner2");
    let beneficiary = addr_make("beneficiary");

    // create a hackatom contract with some funds
    let code_id = app.store_code(hackatom::contract());

    let contract = app
        .instantiate_contract(
            code_id,
            owner.clone(),
            &hackatom::InstantiateMsg {
                beneficiary: beneficiary.as_str().to_owned(),
            },
            &[],
            "Hackatom",
            Some(owner.to_string()),
        )
        .unwrap();

    // check admin set properly
    let info = app.contract_data(&contract).unwrap();
    assert_eq!(info.admin, Some(owner.clone()));

    // transfer admin permissions to owner2
    app.execute(
        owner.clone(),
        CosmosMsg::Wasm(WasmMsg::UpdateAdmin {
            contract_addr: contract.to_string(),
            admin: owner2.to_string(),
        }),
    )
    .unwrap();

    // check admin set properly
    let info = app.contract_data(&contract).unwrap();
    assert_eq!(info.admin, Some(owner2.clone()));

    // update admin fails if not owner2
    app.execute(
        owner.clone(),
        CosmosMsg::Wasm(WasmMsg::UpdateAdmin {
            contract_addr: contract.to_string(),
            admin: owner.to_string(),
        }),
    )
    .unwrap_err();

    // check admin still the same
    let info = app.contract_data(&contract).unwrap();
    assert_eq!(info.admin, Some(owner2));
}

#[test]
fn sent_wasm_migration_works() {
    // The plan:
    // create a hackatom contract with some funds
    // check admin set properly
    // check beneficiary set properly
    // migrate fails if not admin
    // migrate succeeds if admin
    // check beneficiary updated

    // prepare user addresses
    let owner_addr = addr_make("owner");
    let beneficiary_addr = addr_make("beneficiary");
    let random_addr = addr_make("random");

    // set personal balance
    let init_funds = coins(30, "btc");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner_addr, init_funds)
            .unwrap();
    });

    // create a hackatom contract with some funds
    let code_id = app.store_code(hackatom::contract());

    let contract = app
        .instantiate_contract(
            code_id,
            owner_addr.clone(),
            &hackatom::InstantiateMsg {
                beneficiary: beneficiary_addr.as_str().to_owned(),
            },
            &coins(20, "btc"),
            "Hackatom",
            Some(owner_addr.to_string()),
        )
        .unwrap();

    // check admin set properly
    let info = app.contract_data(&contract).unwrap();
    assert_eq!(info.admin, Some(owner_addr.clone()));
    // check beneficiary set properly
    let state: hackatom::InstantiateMsg = app
        .wrap()
        .query_wasm_smart(&contract, &hackatom::QueryMsg::Beneficiary {})
        .unwrap();
    assert_eq!(state.beneficiary, beneficiary_addr.to_string());

    // migrate fails if not admin
    let migrate_msg = hackatom::MigrateMsg {
        new_guy: random_addr.to_string(),
    };
    app.migrate_contract(beneficiary_addr, contract.clone(), &migrate_msg, code_id)
        .unwrap_err();

    // migrate fails if unregistered code id
    app.migrate_contract(
        owner_addr.clone(),
        contract.clone(),
        &migrate_msg,
        code_id + 7,
    )
    .unwrap_err();

    // migrate succeeds when the stars align
    app.migrate_contract(owner_addr, contract.clone(), &migrate_msg, code_id)
        .unwrap();

    // check beneficiary updated
    let state: hackatom::InstantiateMsg = app
        .wrap()
        .query_wasm_smart(&contract, &hackatom::QueryMsg::Beneficiary {})
        .unwrap();
    assert_eq!(state.beneficiary, random_addr.to_string());
}

#[test]
fn sent_funds_properly_visible_on_execution() {
    // Testing if funds on contract are properly visible on contract.
    // Hackatom contract is initialized with 10btc. Then, the contract is executed, with
    // additional 20btc. Then beneficiary balance is checked - expected value is 30btc. 10btc
    // would mean that sending tokens with message is not visible for this very message, and
    // 20btc means, that only such just send funds are visible.

    // prepare user addresses
    let owner_addr = addr_make("owner");
    let beneficiary_addr = addr_make("beneficiary");

    // set personal balance
    let init_funds = coins(30, "btc");
    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner_addr, init_funds)
            .unwrap();
    });

    let code_id = app.store_code(hackatom::contract());

    let contract = app
        .instantiate_contract(
            code_id,
            owner_addr.clone(),
            &hackatom::InstantiateMsg {
                beneficiary: beneficiary_addr.as_str().to_owned(),
            },
            &coins(10, "btc"),
            "Hackatom",
            None,
        )
        .unwrap();

    app.execute_contract(
        owner_addr.clone(),
        contract.clone(),
        &Empty {},
        &coins(20, "btc"),
    )
    .unwrap();

    // Check balance of all accounts to ensure no tokens where burned or created,
    // and they are in correct places
    assert_eq!(get_balance(&app, &owner_addr, "btc"), coin(0, "btc"));
    assert_eq!(get_balance(&app, &contract, "btc"), coin(0, "btc"));
    assert_eq!(get_balance(&app, &beneficiary_addr, "btc"), coin(30, "btc"));
}

/// Demonstrates that we can mint tokens and send from other accounts
/// via a custom module, as an example of ability to do privileged actions.
mod custom_handler {
    use super::*;
    use crate::error::std_error_bail;
    use crate::{BankSudo, BasicAppBuilder};
    use cosmwasm_std::StdError;

    const LOTTERY: Item<Coin> = Item::new("lottery");
    const PITY: Item<Coin> = Item::new("pity");

    #[derive(Clone, Debug, PartialEq, JsonSchema, Serialize, Deserialize)]
    struct CustomLotteryMsg {
        // we mint LOTTERY tokens to this one
        lucky_winner: String,
        // we transfer PITY from lucky_winner to runner_up
        runner_up: String,
    }

    impl CustomMsg for CustomLotteryMsg {}

    struct CustomHandler {}

    impl Module for CustomHandler {
        type ExecT = CustomLotteryMsg;
        type QueryT = Empty;
        type SudoT = Empty;

        fn execute<ExecC, QueryC>(
            &self,
            api: &dyn Api,
            storage: &mut dyn Storage,
            router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
            block: &BlockInfo,
            _sender: Addr,
            msg: Self::ExecT,
        ) -> StdResult<AppResponse>
        where
            ExecC: CustomMsg + DeserializeOwned + 'static,
            QueryC: CustomQuery + DeserializeOwned + 'static,
        {
            let lottery = LOTTERY.load(storage)?;
            let pity = PITY.load(storage)?;

            // mint new tokens
            let mint = BankSudo::Mint {
                to_address: msg.lucky_winner.clone(),
                amount: vec![lottery],
            };
            router.sudo(api, storage, block, mint.into())?;

            // send from an arbitrary account (not the module)
            let transfer = BankMsg::Send {
                to_address: msg.runner_up,
                amount: vec![pity],
            };
            let rcpt = api.addr_validate(&msg.lucky_winner)?;
            router.execute(api, storage, block, rcpt, transfer.into())?;

            Ok(AppResponse::default())
        }

        fn query(
            &self,
            _api: &dyn Api,
            _storage: &dyn Storage,
            _querier: &dyn Querier,
            _block: &BlockInfo,
            _request: Self::QueryT,
        ) -> StdResult<Binary> {
            std_error_bail!("query not implemented for CustomHandler")
        }

        fn sudo<ExecC, QueryC>(
            &self,
            _api: &dyn Api,
            _storage: &mut dyn Storage,
            _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
            _block: &BlockInfo,
            _msg: Self::SudoT,
        ) -> StdResult<AppResponse>
        where
            ExecC: CustomMsg + DeserializeOwned + 'static,
            QueryC: CustomQuery + DeserializeOwned + 'static,
        {
            std_error_bail!("sudo not implemented for CustomHandler")
        }
    }

    impl CustomHandler {
        // this is a custom initialization method
        pub fn set_payout(
            &self,
            storage: &mut dyn Storage,
            lottery: Coin,
            pity: Coin,
        ) -> StdResult<()> {
            LOTTERY.save(storage, &lottery)?;
            PITY.save(storage, &pity)?;
            Ok(())
        }
    }

    // let's call this custom handler
    #[test]
    fn dispatches_messages() {
        // payments. note 54321 - 12321 = 42000
        let denom = "tix";
        let lottery = coin(54321, denom);
        let bonus = coin(12321, denom);

        let mut app = BasicAppBuilder::<CustomLotteryMsg, Empty>::new_custom()
            .with_custom(CustomHandler {})
            .build(|router, _, storage| {
                router
                    .custom
                    .set_payout(storage, lottery.clone(), bonus.clone())
                    .unwrap();
            });

        let winner = app.api().addr_make("winner");
        let second = app.api().addr_make("second");

        // query that balances are empty
        let start = app.wrap().query_balance(&winner, denom).unwrap();
        assert_eq!(start, coin(0, denom));

        // trigger the custom module
        let msg = CosmosMsg::Custom(CustomLotteryMsg {
            lucky_winner: winner.to_string(),
            runner_up: second.to_string(),
        });
        let anyone = app.api().addr_make("anyone");
        app.execute(anyone, msg).unwrap();

        // see if coins were properly added
        let big_win = app.wrap().query_balance(&winner, denom).unwrap();
        assert_eq!(big_win, coin(42000, denom));
        let little_win = app.wrap().query_balance(&second, denom).unwrap();
        assert_eq!(little_win, bonus);
    }
}

mod reply_data_overwrite {
    use super::*;

    fn make_echo_exec_msg(
        contract_addr: Addr,
        data: impl Into<Option<&'static str>>,
        sub_msg: Vec<SubMsg>,
    ) -> CosmosMsg {
        let data = data.into().map(|s| s.to_string());
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.into(),
            msg: to_json_binary(&echo::ExecMessage {
                data,
                sub_msg,
                ..Default::default()
            })
            .unwrap(),
            funds: vec![],
        })
    }

    fn make_echo_reply_always_submsg(
        contract_addr: Addr,
        data: impl Into<Option<&'static str>>,
        sub_msg: Vec<SubMsg>,
        id: u64,
    ) -> SubMsg {
        SubMsg::reply_always(make_echo_exec_msg(contract_addr, data, sub_msg), id)
    }

    fn make_echo_reply_never_submsg(
        contract_addr: Addr,
        data: impl Into<Option<&'static str>>,
        sub_msg: Vec<SubMsg>,
    ) -> SubMsg {
        SubMsg::reply_never(make_echo_exec_msg(contract_addr, data, sub_msg))
    }

    #[test]
    fn no_submsg() {
        // create a chain with default settings
        let mut chain = App::default();

        // prepare the owner address
        let owner = chain.api().addr_make("owner");

        // store the echo contract on chain
        let echo_code_id = chain.store_code(echo::contract());

        // instantiate the echo contract
        let echo_contract_addr = chain
            .instantiate_contract(echo_code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        // prepare the message to be executed by echo contract
        // send only data payload without any submessages
        let echo_exec_msg = echo::ExecMessage::<Empty> {
            data: "PAYLOAD".to_string().into(),
            ..Default::default()
        };

        // execute the message
        let response = chain
            .execute_contract(owner, echo_contract_addr, &echo_exec_msg, &[])
            .unwrap();

        // the returned data should be the same as the one being previously sent
        assert_eq!(response.data, Some(b"PAYLOAD".into()));
    }

    #[test]
    fn single_submsg() {
        // create a chain with default settings
        let mut chain = App::default();

        // prepare the owner address
        let owner = chain.api().addr_make("owner");

        // store the echo contract on chain
        let echo_code_id = chain.store_code(echo::contract());

        // instantiate the echo contract
        let echo_contract_addr = chain
            .instantiate_contract(echo_code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        // prepare the message to be executed by echo contract
        let echo_exec_msg = echo::ExecMessage::<Empty> {
            data: "FIRST".to_string().into(),
            sub_msg: vec![make_echo_reply_always_submsg(
                echo_contract_addr.clone(),
                "SECOND",
                vec![],
                EXECUTE_REPLY_BASE_ID,
            )],
            ..Default::default()
        };

        // execute the message
        let response = chain
            .execute_contract(owner, echo_contract_addr, &echo_exec_msg, &[])
            .unwrap();

        // the returned data should be the data payload of the submessage
        assert_eq!(response.data, Some(b"SECOND".into()));
    }

    #[test]
    fn single_submsg_no_reply() {
        // create a chain with default settings
        let mut chain = App::default();

        // prepare the owner address
        let owner = chain.api().addr_make("owner");

        // store the echo contract on chain
        let echo_code_id = chain.store_code(echo::contract());

        // instantiate the echo contract
        let echo_contract_addr = chain
            .instantiate_contract(echo_code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        // prepare the message to be executed by echo contract
        let echo_exec_msg = echo::ExecMessage::<Empty> {
            data: "FIRST".to_string().into(),
            sub_msg: vec![make_echo_reply_never_submsg(
                echo_contract_addr.clone(),
                "SECOND",
                vec![],
            )],
            ..Default::default()
        };

        // execute the message
        let response = chain
            .execute_contract(owner, echo_contract_addr, &echo_exec_msg, &[])
            .unwrap();

        // the returned data should be the data payload of the original message
        assert_eq!(response.data, Some(b"FIRST".into()));
    }

    #[test]
    fn single_no_submsg_data() {
        // create a chain with default settings
        let mut chain = App::default();

        // prepare the owner address
        let owner = chain.api().addr_make("owner");

        // store the echo contract on chain
        let echo_code_id = chain.store_code(echo::contract());

        let echo_contract_addr = chain
            .instantiate_contract(echo_code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let echo_exec_msg = echo::ExecMessage {
            data: "FIRST".to_string().into(),
            sub_msg: vec![make_echo_reply_always_submsg(
                echo_contract_addr.clone(),
                None,
                vec![],
                1,
            )],
            ..Default::default()
        };

        let response = chain
            .execute_contract(owner, echo_contract_addr, &echo_exec_msg, &[])
            .unwrap();

        assert_eq!(response.data, Some(b"FIRST".into()));
    }

    #[test]
    fn single_no_top_level_data() {
        // create a chain with default settings
        let mut chain = App::default();

        let owner = chain.api().addr_make("owner");

        let echo_code_id = chain.store_code(echo::contract());

        let echo_contract_addr = chain
            .instantiate_contract(echo_code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let echo_exec_msg = echo::ExecMessage {
            data: None,
            sub_msg: vec![make_echo_reply_always_submsg(
                echo_contract_addr.clone(),
                "SECOND",
                vec![],
                EXECUTE_REPLY_BASE_ID,
            )],
            ..Default::default()
        };

        let response = chain
            .execute_contract(owner, echo_contract_addr, &echo_exec_msg, &[])
            .unwrap();

        assert_eq!(response.data, Some(b"SECOND".into()));
    }

    #[test]
    fn single_submsg_reply_returns_none() {
        // create a chain with default settings
        let mut chain = App::default();

        // prepare user addresses
        let owner = addr_make("owner");

        // store reflect contract on chain
        let reflect_code_id = chain.store_code(reflect::contract());

        // instantiate reflect contract
        let reflect_contract_addr = chain
            .instantiate_contract(
                reflect_code_id,
                owner.clone(),
                &Empty {},
                &[],
                "Reflect",
                None,
            )
            .unwrap();

        // store the echo contract on chain
        let echo_code_id = chain.store_code(echo::contract());

        // instantiate the echo contract
        let echo_contract_addr = chain
            .instantiate_contract(echo_code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        // firstly reflect contract will call echo contract, then the echo contract will return the data,
        // but there is no submessage, so no reply entrypoint of reflect contract will be called,
        // finally the top-level app (this test) will not display any data

        // prepare the echo execute message
        let echo_msg = echo::ExecMessage::<Empty> {
            data: Some("ORIGINAL".into()),
            ..Default::default()
        };

        // prepare reflect execute message
        let reflect_msg = reflect::ExecMessage::<Empty> {
            sub_msg: vec![SubMsg::reply_never(WasmMsg::Execute {
                contract_addr: echo_contract_addr.to_string(),
                msg: to_json_binary(&echo_msg).unwrap(),
                funds: vec![],
            })],
        };

        // execute reflect message
        let response = chain
            .execute_contract(owner, reflect_contract_addr.clone(), &reflect_msg, &[])
            .unwrap();

        // ensure the data in response is empty
        assert_eq!(response.data, None);
        // ensure expected events are returned
        assert_eq!(response.events.len(), 2);
        let make_event = |contract_addr: &Addr| {
            Event::new("execute").add_attribute("_contract_address", contract_addr)
        };
        response.assert_event(&make_event(&reflect_contract_addr));
        response.assert_event(&make_event(&echo_contract_addr));
    }

    #[test]
    fn multiple_submsg() {
        // create a chain with default settings
        let mut chain = App::default();

        // prepare user addresses
        let owner = chain.api().addr_make("owner");

        let echo_code_id = chain.store_code(echo::contract());

        let echo_contract_addr = chain
            .instantiate_contract(echo_code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let response = chain
            .execute_contract(
                owner,
                echo_contract_addr.clone(),
                &echo::ExecMessage {
                    data: "ORIGINAL".to_string().into(),
                    sub_msg: vec![
                        make_echo_reply_always_submsg(
                            echo_contract_addr.clone(),
                            None,
                            vec![],
                            EXECUTE_REPLY_BASE_ID + 1,
                        ),
                        make_echo_reply_always_submsg(
                            echo_contract_addr.clone(),
                            "FIRST",
                            vec![],
                            EXECUTE_REPLY_BASE_ID + 2,
                        ),
                        make_echo_reply_always_submsg(
                            echo_contract_addr.clone(),
                            "SECOND",
                            vec![],
                            EXECUTE_REPLY_BASE_ID + 3,
                        ),
                        make_echo_reply_always_submsg(
                            echo_contract_addr,
                            None,
                            vec![],
                            EXECUTE_REPLY_BASE_ID + 4,
                        ),
                    ],
                    ..Default::default()
                },
                &[],
            )
            .unwrap();

        assert_eq!(response.data, Some(b"SECOND".into()));
    }

    #[test]
    fn multiple_submsg_no_reply() {
        let mut app = App::default();

        let owner = app.api().addr_make("owner");

        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let response = app
            .execute_contract(
                owner,
                contract.clone(),
                &echo::ExecMessage {
                    data: "ORIGINAL".to_string().into(),
                    sub_msg: vec![
                        make_echo_reply_never_submsg(contract.clone(), None, vec![]),
                        make_echo_reply_never_submsg(contract.clone(), "FIRST", vec![]),
                        make_echo_reply_never_submsg(contract.clone(), "SECOND", vec![]),
                        make_echo_reply_never_submsg(contract, None, vec![]),
                    ],
                    ..Default::default()
                },
                &[],
            )
            .unwrap();

        assert_eq!(response.data, Some(b"ORIGINAL".into()));
    }

    #[test]
    fn multiple_submsg_mixed() {
        let mut chain = App::default();

        let owner = chain.api().addr_make("owner");

        let echo_code_id = chain.store_code(echo::contract());

        let echo_contract_addr = chain
            .instantiate_contract(echo_code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let response = chain
            .execute_contract(
                owner,
                echo_contract_addr.clone(),
                &echo::ExecMessage {
                    sub_msg: vec![
                        make_echo_reply_always_submsg(
                            echo_contract_addr.clone(),
                            None,
                            vec![],
                            EXECUTE_REPLY_BASE_ID + 1,
                        ),
                        make_echo_reply_never_submsg(echo_contract_addr.clone(), "FIRST", vec![]),
                        make_echo_reply_always_submsg(
                            echo_contract_addr.clone(),
                            "SECOND",
                            vec![],
                            EXECUTE_REPLY_BASE_ID + 2,
                        ),
                        make_echo_reply_always_submsg(
                            echo_contract_addr.clone(),
                            None,
                            vec![],
                            EXECUTE_REPLY_BASE_ID + 3,
                        ),
                        make_echo_reply_never_submsg(echo_contract_addr, "THIRD", vec![]),
                    ],
                    ..Default::default()
                },
                &[],
            )
            .unwrap();

        assert_eq!(response.data, Some(b"SECOND".into()));
    }

    #[test]
    fn nested_submsg() {
        let mut chain = App::default();

        let owner = chain.api().addr_make("owner");

        let echo_code_id = chain.store_code(echo::contract());

        let echo_contract_addr = chain
            .instantiate_contract(echo_code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let response = chain
            .execute_contract(
                owner,
                echo_contract_addr.clone(),
                &echo::ExecMessage {
                    data: "ORIGINAL".to_string().into(),
                    sub_msg: vec![make_echo_reply_always_submsg(
                        echo_contract_addr.clone(),
                        None,
                        vec![make_echo_reply_always_submsg(
                            echo_contract_addr.clone(),
                            "FIRST",
                            vec![make_echo_reply_always_submsg(
                                echo_contract_addr.clone(),
                                "SECOND",
                                vec![make_echo_reply_always_submsg(
                                    echo_contract_addr,
                                    None,
                                    vec![],
                                    EXECUTE_REPLY_BASE_ID + 4,
                                )],
                                EXECUTE_REPLY_BASE_ID + 3,
                            )],
                            EXECUTE_REPLY_BASE_ID + 2,
                        )],
                        EXECUTE_REPLY_BASE_ID + 1,
                    )],
                    ..Default::default()
                },
                &[],
            )
            .unwrap();

        assert_eq!(response.data, Some(b"SECOND".into()));
    }
}

mod response_validation {
    use super::*;

    #[test]
    fn empty_attribute_key() {
        let mut app = App::default();

        let owner = app.api().addr_make("owner");

        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let err = app
            .execute_contract(
                owner,
                contract,
                &echo::ExecMessage::<Empty> {
                    data: None,
                    attributes: vec![
                        Attribute::new("   ", "value"),
                        Attribute::new("proper", "proper_val"),
                    ],
                    ..Default::default()
                },
                &[],
            )
            .unwrap_err();

        let err_str = err.to_string();
        assert!(
            err_str.starts_with("kind: Other, error: Error executing WasmMsg")
                && err_str.contains("Empty attribute key. Value: value")
        );
    }

    #[test]
    fn empty_attribute_value_should_work() {
        let mut app = App::default();

        let owner = app.api().addr_make("owner");
        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        assert!(app
            .execute_contract(
                owner,
                contract,
                &echo::ExecMessage::<Empty> {
                    data: None,
                    attributes: vec![
                        Attribute::new("key", "   "),
                        Attribute::new("proper", "proper_val"),
                    ],
                    ..Default::default()
                },
                &[],
            )
            .is_ok());
    }

    #[test]
    fn empty_event_attribute_key() {
        let mut app = App::default();

        let owner = app.api().addr_make("owner");

        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let err = app
            .execute_contract(
                owner,
                contract,
                &echo::ExecMessage::<Empty> {
                    data: None,
                    events: vec![Event::new("event")
                        .add_attribute("   ", "value")
                        .add_attribute("proper", "proper_val")],
                    ..Default::default()
                },
                &[],
            )
            .unwrap_err();

        let err_str = err.to_string();
        assert!(
            err_str.starts_with("kind: Other, error: Error executing WasmMsg")
                && err_str.contains("Empty attribute key. Value: value")
        );
    }

    #[test]
    fn empty_event_attribute_value_should_work() {
        let mut app = App::default();

        let owner = app.api().addr_make("owner");
        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        assert!(app
            .execute_contract(
                owner,
                contract,
                &echo::ExecMessage::<Empty> {
                    data: None,
                    events: vec![Event::new("event")
                        .add_attribute("key", "   ")
                        .add_attribute("proper", "proper_val")],
                    ..Default::default()
                },
                &[],
            )
            .is_ok());
    }

    #[test]
    fn too_short_event_type() {
        let mut app = App::default();

        let owner = app.api().addr_make("owner");

        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let err = app
            .execute_contract(
                owner,
                contract,
                &echo::ExecMessage::<Empty> {
                    data: None,
                    events: vec![Event::new(" e "), Event::new("event")],
                    ..Default::default()
                },
                &[],
            )
            .unwrap_err();

        let err_str = err.to_string();
        assert!(
            err_str.starts_with("kind: Other, error: Error executing WasmMsg")
                && err_str.contains("Event type too short: e")
        );
    }
}

mod contract_instantiation {

    #[test]
    #[cfg(feature = "cosmwasm_1_2")]
    fn instantiate2_works() {
        use super::*;

        // prepare application and actors
        let mut app = App::default();
        let sender = app.api().addr_make("sender");
        let creator = app.api().addr_make("creator");

        // store contract's code
        let code_id = app.store_code_with_creator(creator, echo::contract());

        // initialize the contract
        let init_msg = to_json_binary(&Empty {}).unwrap();
        let salt = cosmwasm_std::HexBinary::from_hex("010203040506").unwrap();
        let msg = WasmMsg::Instantiate2 {
            admin: None,
            code_id,
            msg: init_msg,
            funds: vec![],
            label: "label".into(),
            salt: salt.into(),
        };
        let res = app.execute(sender, msg.into()).unwrap();

        // assert a proper instantiate result
        let parsed = parse_instantiate_response_data(res.data.unwrap().as_slice()).unwrap();
        assert!(parsed.data.is_none());

        // assert contract's address is exactly the predicted one,
        // in default address generator, this is like `contract` + salt in hex
        assert_eq!(
            parsed.contract_address,
            "cosmwasm167g7x7auj3l00lhdcevusncx565ytz6a6xvmx2f5xuy84re9ddrqczpzkm",
        );
    }
}

mod wasm_queries {

    #[test]
    #[cfg(feature = "cosmwasm_1_2")]
    fn query_existing_code_info() {
        use super::*;
        let mut app = App::default();
        let creator = app.api().addr_make("creator");
        let code_id = app.store_code_with_creator(creator.clone(), echo::contract());
        let code_info_response = app.wrap().query_wasm_code_info(code_id).unwrap();
        assert_eq!(code_id, code_info_response.code_id);
        assert_eq!(creator.to_string(), code_info_response.creator.to_string());
        assert_eq!(32, code_info_response.checksum.as_slice().len());
    }

    #[test]
    #[cfg(feature = "cosmwasm_1_2")]
    fn query_non_existing_code_info() {
        use super::*;
        let app = App::default();
        assert_eq!(
            "kind: Other, error: Querier contract error: kind: Other, error: code id: invalid",
            app.wrap().query_wasm_code_info(0).unwrap_err().to_string()
        );
        assert_eq!(
            "kind: Other, error: Querier contract error: kind: Other, error: code id 1: no such code",
            app.wrap().query_wasm_code_info(1).unwrap_err().to_string()
        );
    }
}

mod custom_messages {
    use super::*;

    #[test]
    fn triggering_custom_msg() {
        let custom_handler = CachingCustomHandler::<CustomHelperMsg, Empty>::default();
        let custom_handler_state = custom_handler.state();

        let mut app = AppBuilder::new_custom()
            .with_custom(custom_handler)
            .build(no_init);

        let sender = app.api().addr_make("sender");
        let owner = app.api().addr_make("owner");

        let contract_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(contract_id, owner, &Empty {}, &[], "Echo", None)
            .unwrap();

        app.execute_contract(
            sender,
            contract,
            &echo::ExecMessage {
                sub_msg: vec![SubMsg::new(CosmosMsg::Custom(CustomHelperMsg::SetAge {
                    age: 20,
                }))],
                ..Default::default()
            },
            &[],
        )
        .unwrap();

        assert_eq!(
            custom_handler_state.execs().to_owned(),
            vec![CustomHelperMsg::SetAge { age: 20 }]
        );

        assert!(custom_handler_state.queries().is_empty());
    }
}

mod protobuf_wrapped_data {
    use super::*;
    use crate::BasicApp;

    #[test]
    fn instantiate_wrapped_properly() {
        // prepare user addresses
        let owner = addr_make("owner");

        // set personal balance
        let init_funds = vec![coin(20, "btc")];
        let mut app = custom_app::<CustomHelperMsg, Empty, _>(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &owner, init_funds)
                .unwrap();
        });

        // set up reflect contract
        let code_id = app.store_code(reflect::contract());

        let init_msg = to_json_binary(&Empty {}).unwrap();
        let msg = WasmMsg::Instantiate {
            admin: None,
            code_id,
            msg: init_msg,
            funds: vec![],
            label: "label".into(),
        };
        let res = app.execute(owner, msg.into()).unwrap();

        // assert we have a proper instantiate result
        let parsed = parse_instantiate_response_data(res.data.unwrap().as_slice()).unwrap();
        assert!(parsed.data.is_none());
        // check the address is right

        let count: payout::CountResponse = app
            .wrap()
            .query_wasm_smart(&parsed.contract_address, &reflect::QueryMessage::Count)
            .unwrap();
        assert_eq!(count.count, 0);
    }

    #[test]
    fn instantiate_with_data_works() {
        let mut app = BasicApp::new(no_init);

        let owner = app.api().addr_make("owner");

        // set up echo contract
        let code_id = app.store_code(echo::contract());

        let msg = echo::InitMessage::<Empty> {
            data: Some("food".into()),
            sub_msg: None,
        };
        let init_msg = to_json_binary(&msg).unwrap();
        let msg = WasmMsg::Instantiate {
            admin: None,
            code_id,
            msg: init_msg,
            funds: vec![],
            label: "label".into(),
        };
        let res = app.execute(owner, msg.into()).unwrap();

        // assert we have a proper instantiate result
        let parsed = parse_instantiate_response_data(res.data.unwrap().as_slice()).unwrap();
        assert!(parsed.data.is_some());
        assert_eq!(parsed.data.unwrap(), Binary::from(b"food"));
        assert!(!parsed.contract_address.is_empty());
    }

    #[test]
    fn instantiate_with_reply_works() {
        let mut app = BasicApp::new(no_init);

        let owner = app.api().addr_make("owner");

        // set up echo contract
        let code_id = app.store_code(echo::contract());

        let msg = echo::InitMessage::<Empty> {
            data: Some("food".into()),
            ..Default::default()
        };
        let addr1 = app
            .instantiate_contract(code_id, owner.clone(), &msg, &[], "first", None)
            .unwrap();

        // another echo contract
        let msg = echo::ExecMessage::<Empty> {
            data: Some("Passed to contract instantiation, returned as reply, and then returned as response".into()),
            ..Default::default()
        };
        let sub_msg = SubMsg::reply_on_success(
            WasmMsg::Execute {
                contract_addr: addr1.to_string(),
                msg: to_json_binary(&msg).unwrap(),
                funds: vec![],
            },
            EXECUTE_REPLY_BASE_ID,
        );
        let init_msg = echo::InitMessage::<Empty> {
            data: Some("Overwrite me".into()),
            sub_msg: Some(vec![sub_msg]),
        };
        let init_msg = to_json_binary(&init_msg).unwrap();
        let msg = WasmMsg::Instantiate {
            admin: None,
            code_id,
            msg: init_msg,
            funds: vec![],
            label: "label".into(),
        };
        let res = app.execute(owner, msg.into()).unwrap();

        // assert we have a proper instantiate result
        let parsed = parse_instantiate_response_data(res.data.unwrap().as_slice()).unwrap();
        assert!(parsed.data.is_some());
        // Result is from the reply, not the original one
        assert_eq!(parsed.data.unwrap(), Binary::from(b"Passed to contract instantiation, returned as reply, and then returned as response"));
        assert!(!parsed.contract_address.is_empty());
        assert_ne!(parsed.contract_address, addr1.to_string());
    }

    #[test]
    fn execute_wrapped_properly() {
        let mut app = BasicApp::new(no_init);

        let owner = app.api().addr_make("owner");

        // set up reflect contract
        let code_id = app.store_code(echo::contract());

        let echo_addr = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "label", None)
            .unwrap();

        // ensure message has the same wrapper as it should
        let msg = echo::ExecMessage::<Empty> {
            data: Some("hello".into()),
            ..echo::ExecMessage::default()
        };
        // execute_contract now decodes a protobuf wrapper, so we get the top-level response
        let exec_res = app.execute_contract(owner, echo_addr, &msg, &[]).unwrap();
        assert_eq!(exec_res.data, Some(Binary::from(b"hello")));
    }
}

mod errors {
    use super::*;

    #[test]
    fn simple_instantiation() {
        let mut app = App::default();

        let owner = app.api().addr_make("owner");

        // set up contract
        let code_id = app.store_code(error::contract(false));

        let msg = Empty {};
        let err = app
            .instantiate_contract(code_id, owner, &msg, &[], "error", None)
            .unwrap_err();

        let err_str = err.to_string();
        assert!(
            err_str.starts_with("kind: Other, error: Error executing WasmMsg")
                && err_str.contains("Init failed")
        );
    }

    #[test]
    fn simple_call() {
        let mut app = App::default();

        let owner = app.api().addr_make("owner");
        let random_addr = app.api().addr_make("random");

        // set up contract
        let code_id = app.store_code(error::contract(true));

        let msg = Empty {};
        let contract_addr = app
            .instantiate_contract(code_id, owner, &msg, &[], "error", None)
            .unwrap();

        // execute should error
        let err = app
            .execute_contract(random_addr, contract_addr, &msg, &[])
            .unwrap_err();

        let err_str = err.to_string();
        assert!(
            err_str.starts_with("kind: Other, error: Error executing WasmMsg")
                && err_str.contains("Handle failed")
        );
    }

    #[test]
    fn nested_call() {
        let mut app = App::default();

        let owner = app.api().addr_make("owner");
        let random_addr = app.api().addr_make("random");

        let error_code_id = app.store_code(error::contract(true));
        let caller_code_id = app.store_code(caller::contract());

        // set up contract_helpers
        let msg = Empty {};
        let caller_addr = app
            .instantiate_contract(caller_code_id, owner.clone(), &msg, &[], "caller", None)
            .unwrap();
        let error_addr = app
            .instantiate_contract(error_code_id, owner, &msg, &[], "error", None)
            .unwrap();

        // execute should error
        let msg = WasmMsg::Execute {
            contract_addr: error_addr.into(),
            msg: to_json_binary(&Empty {}).unwrap(),
            funds: vec![],
        };
        let err = app
            .execute_contract(random_addr, caller_addr, &msg, &[])
            .unwrap_err();

        let err_str = err.to_string();
        assert!(
            err_str.starts_with("kind: Other, error: Error executing WasmMsg")
                && err_str.contains("Handle failed")
        );
    }

    #[test]
    fn double_nested_call() {
        let mut app = App::default();

        let owner_addr = app.api().addr_make("owner");
        let random_addr = app.api().addr_make("random");

        let error_code_id = app.store_code(error::contract(true));
        let caller_code_id = app.store_code(caller::contract());

        // set up contract_helpers
        let msg = Empty {};
        let caller_addr1 = app
            .instantiate_contract(
                caller_code_id,
                owner_addr.clone(),
                &msg,
                &[],
                "caller",
                None,
            )
            .unwrap();
        let caller_addr2 = app
            .instantiate_contract(
                caller_code_id,
                owner_addr.clone(),
                &msg,
                &[],
                "caller",
                None,
            )
            .unwrap();
        let error_addr = app
            .instantiate_contract(error_code_id, owner_addr, &msg, &[], "error", None)
            .unwrap();

        // caller1 calls caller2, caller2 calls error
        let msg = WasmMsg::Execute {
            contract_addr: caller_addr2.into(),
            msg: to_json_binary(&WasmMsg::Execute {
                contract_addr: error_addr.into(),
                msg: to_json_binary(&Empty {}).unwrap(),
                funds: vec![],
            })
            .unwrap(),
            funds: vec![],
        };
        let err = app
            .execute_contract(random_addr, caller_addr1, &msg, &[])
            .unwrap_err();

        let err_str = err.to_string();
        assert!(
            err_str.starts_with("kind: Other, error: Error executing WasmMsg")
                && err_str.contains("Handle failed")
        );
    }
}
