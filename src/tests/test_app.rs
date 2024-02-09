use crate::custom_handler::CachingCustomHandler;
use crate::error::{bail, AnyResult};
use crate::test_helpers::echo::EXECUTE_REPLY_BASE_ID;
use crate::test_helpers::{caller, echo, error, hackatom, payout, reflect, CustomHelperMsg};
use crate::transactions::{transactional, StorageTransaction};
use crate::wasm::ContractData;
use crate::AppBuilder;
use crate::{
    custom_app, next_block, no_init, App, AppResponse, Bank, CosmosRouter, Distribution, Executor,
    Module, Router, Staking, Wasm, WasmSudo,
};
use cosmwasm_std::testing::{mock_env, MockApi, MockQuerier};
use cosmwasm_std::{
    coin, coins, from_json, to_json_binary, Addr, AllBalanceResponse, Api, Attribute, BankMsg,
    BankQuery, Binary, BlockInfo, Coin, CosmosMsg, CustomMsg, CustomQuery, Empty, Event,
    OverflowError, OverflowOperation, Querier, Reply, StdError, StdResult, Storage, SubMsg,
    WasmMsg,
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
    addr: &Addr,
) -> Vec<Coin>
where
    CustomT::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    ApiT: Api,
    StorageT: Storage,
    CustomT: Module,
{
    app.wrap().query_all_balances(addr).unwrap()
}

fn query_router<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>(
    router: &Router<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>,
    api: &dyn Api,
    storage: &dyn Storage,
    rcpt: &Addr,
) -> Vec<Coin>
where
    CustomT::ExecT: CustomMsg,
    CustomT::QueryT: CustomQuery + DeserializeOwned,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
    BankT: Bank,
    CustomT: Module,
    StakingT: Staking,
    DistrT: Distribution,
{
    let query = BankQuery::AllBalances {
        address: rcpt.into(),
    };
    let block = mock_env().block;
    let querier: MockQuerier<CustomT::QueryT> = MockQuerier::new(&[]);
    let res = router
        .bank
        .query(api, storage, &querier, &block, query)
        .unwrap();
    let val: AllBalanceResponse = from_json(res).unwrap();
    val.amount
}

fn query_app<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT>(
    app: &App<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT>,
    rcpt: &Addr,
) -> Vec<Coin>
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
    let query = BankQuery::AllBalances {
        address: rcpt.into(),
    }
    .into();
    let val: AllBalanceResponse = app.wrap().query(&query).unwrap();
    val.amount
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
    let api = MockApi::default();
    let owner_addr = api.addr_make("owner");
    let recipient_addr = api.addr_make("recipient");

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
    let cached_rcpt = query_router(app.router(), app.api(), &cache, &recipient_addr);
    assert_eq!(coins(25, "eth"), cached_rcpt);
    let router_rcpt = query_app(&app, &recipient_addr);
    assert_eq!(router_rcpt, vec![]);

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
        let cached_rcpt = query_router(app.router(), app.api(), read, &recipient_addr);
        assert_eq!(coins(25, "eth"), cached_rcpt);
        let cached2_rcpt = query_router(app.router(), app.api(), cache2, &recipient_addr);
        assert_eq!(coins(37, "eth"), cached2_rcpt);
        Ok(())
    })
    .unwrap();

    // apply first to router
    cache.prepare().commit(app.storage_mut());

    let committed = query_app(&app, &recipient_addr);
    assert_eq!(coins(37, "eth"), committed);
}

#[test]
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
    let api = MockApi::default();
    let owner_addr = api.addr_make("owner");
    let recipient_addr = api.addr_make("recipient");

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
    let rich = get_balance(&app, &owner_addr);
    assert_eq!(vec![coin(15, "btc"), coin(70, "eth")], rich);
    let poor = get_balance(&app, &recipient_addr);
    assert_eq!(vec![coin(10, "btc"), coin(30, "eth")], poor);

    // can send from other account (but funds will be deducted from sender)
    app.execute(recipient_addr.clone(), msg).unwrap();

    // cannot send too much
    let msg = BankMsg::Send {
        to_address: recipient_addr.into(),
        amount: coins(20, "btc"),
    }
    .into();
    app.execute(owner_addr.clone(), msg).unwrap_err();

    let rich = get_balance(&app, &owner_addr);
    assert_eq!(vec![coin(15, "btc"), coin(70, "eth")], rich);
}

#[test]
fn simple_contract() {
    // prepare user addresses
    let api = MockApi::default();
    let owner_addr = api.addr_make("owner");

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
    let contract_address = app
        .instantiate_contract(
            code_id,
            owner_addr.clone(),
            &msg,
            &coins(23, "eth"),
            "Payout",
            None,
        )
        .unwrap();

    let contract_data = app.contract_data(&contract_address).unwrap();
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
    let sender = get_balance(&app, &owner_addr);
    assert_eq!(sender, vec![coin(20, "btc"), coin(77, "eth")]);
    // get contract address, has funds
    let funds = get_balance(&app, &contract_address);
    assert_eq!(funds, coins(23, "eth"));

    // create empty account
    let random_address = app.api().addr_make("random");
    let funds = get_balance(&app, &random_address);
    assert_eq!(funds, vec![]);

    // do one payout and see money coming in
    let res = app
        .execute_contract(
            random_address.clone(),
            contract_address.clone(),
            &Empty {},
            &[],
        )
        .unwrap();
    assert_eq!(3, res.events.len());

    // the call to payout does emit this as well as custom attributes
    let payout_exec = &res.events[0];
    assert_eq!(payout_exec.ty.as_str(), "execute");
    assert_eq!(
        payout_exec.attributes,
        [("_contract_address", &contract_address)]
    );

    // next is a custom wasm event
    let custom_attrs = res.custom_attrs(1);
    assert_eq!(custom_attrs, [("action", "payout")]);

    // then the transfer event
    let expected_transfer = Event::new("transfer")
        .add_attribute("recipient", &random_address)
        .add_attribute("sender", &contract_address)
        .add_attribute("amount", "5eth");
    assert_eq!(&expected_transfer, &res.events[2]);

    // random got cash
    let funds = get_balance(&app, &random_address);
    assert_eq!(funds, coins(5, "eth"));
    // contract lost it
    let funds = get_balance(&app, &contract_address);
    assert_eq!(funds, coins(18, "eth"));
}

#[test]
fn reflect_success() {
    // prepare user addresses
    let api = MockApi::default();
    let owner_addr = api.addr_make("owner");
    let random_addr = api.addr_make("random");

    // set personal balance
    let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
    let mut app = custom_app::<CustomHelperMsg, Empty, _>(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner_addr, init_funds)
            .unwrap();
    });

    // set up payout contract
    let payout_id = app.store_code(payout::contract());

    let msg = payout::InstantiateMessage {
        payout: coin(5, "eth"),
    };
    let payout_addr = app
        .instantiate_contract(
            payout_id,
            owner_addr.clone(),
            &msg,
            &coins(23, "eth"),
            "Payout",
            None,
        )
        .unwrap();

    // set up reflect contract
    let reflect_id = app.store_code(reflect::contract());

    let reflect_addr = app
        .instantiate_contract(reflect_id, owner_addr, &Empty {}, &[], "Reflect", None)
        .unwrap();

    // reflect account is empty
    let funds = get_balance(&app, &reflect_addr);
    assert_eq!(funds, vec![]);
    // reflect count is 1
    let query_res: payout::CountResponse = app
        .wrap()
        .query_wasm_smart(&reflect_addr, &reflect::QueryMsg::Count {})
        .unwrap();
    assert_eq!(0, query_res.count);

    // reflecting payout message pays reflect contract
    let msg = SubMsg::new(WasmMsg::Execute {
        contract_addr: payout_addr.clone().into(),
        msg: b"{}".into(),
        funds: vec![],
    });
    let msgs = reflect::Message {
        messages: vec![msg],
    };
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
    let funds = get_balance(&app, &reflect_addr);
    assert_eq!(funds, coins(5, "eth"));

    // reflect count updated
    let query_res: payout::CountResponse = app
        .wrap()
        .query_wasm_smart(&reflect_addr, &reflect::QueryMsg::Count {})
        .unwrap();
    assert_eq!(1, query_res.count);
}

#[test]
fn reflect_error() {
    // set personal balance
    let api = MockApi::default();
    let owner = api.addr_make("owner");
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
    let funds = get_balance(&app, &reflect_addr);
    assert_eq!(funds, coins(40, "eth"));
    let random_address = app.api().addr_make("random");

    // sending 7 eth works
    let msg = SubMsg::new(BankMsg::Send {
        to_address: random_address.clone().into(),
        amount: coins(7, "eth"),
    });
    let msgs = reflect::Message {
        messages: vec![msg],
    };
    let res = app
        .execute_contract(random_address.clone(), reflect_addr.clone(), &msgs, &[])
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
    let funds = get_balance(&app, &random_address);
    assert_eq!(funds, coins(7, "eth"));

    // reflect count should be updated to 1
    let query_res: payout::CountResponse = app
        .wrap()
        .query_wasm_smart(&reflect_addr, &reflect::QueryMsg::Count {})
        .unwrap();
    assert_eq!(1, query_res.count);

    // sending 8 eth, then 3 btc should fail both
    let msg = SubMsg::new(BankMsg::Send {
        to_address: random_address.clone().into(),
        amount: coins(8, "eth"),
    });
    let msg2 = SubMsg::new(BankMsg::Send {
        to_address: random_address.clone().into(),
        amount: coins(3, "btc"),
    });
    let msgs = reflect::Message {
        messages: vec![msg, msg2],
    };
    let err = app
        .execute_contract(random_address.clone(), reflect_addr.clone(), &msgs, &[])
        .unwrap_err();
    assert_eq!(
        StdError::overflow(OverflowError::new(OverflowOperation::Sub)),
        err.downcast().unwrap()
    );

    // first one should have been rolled-back on error (no second payment)
    let funds = get_balance(&app, &random_address);
    assert_eq!(funds, coins(7, "eth"));

    // failure should not update reflect count
    let query_res: payout::CountResponse = app
        .wrap()
        .query_wasm_smart(&reflect_addr, &reflect::QueryMsg::Count {})
        .unwrap();
    assert_eq!(1, query_res.count);
}

#[test]
fn sudo_works() {
    let owner = Addr::unchecked("owner");
    let init_funds = vec![coin(100, "eth")];

    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner, init_funds)
            .unwrap();
    });

    let payout_id = app.store_code(payout::contract());

    let msg = payout::InstantiateMessage {
        payout: coin(5, "eth"),
    };
    let payout_addr = app
        .instantiate_contract(payout_id, owner, &msg, &coins(23, "eth"), "Payout", None)
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
        msg: to_json_binary(&msg).unwrap(),
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
    // set personal balance
    let owner = Addr::unchecked("owner");
    let random = Addr::unchecked("random");
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

    // no reply writen beforehand
    let query = reflect::QueryMsg::Reply { id: 123 };
    let res: StdResult<Reply> = app.wrap().query_wasm_smart(&reflect_addr, &query);
    res.unwrap_err();

    // reflect sends 7 eth, success
    let msg = SubMsg::reply_always(
        BankMsg::Send {
            to_address: random.clone().into(),
            amount: coins(7, "eth"),
        },
        123,
    );
    let msgs = reflect::Message {
        messages: vec![msg],
    };
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
    let msg = SubMsg::reply_always(
        BankMsg::Send {
            to_address: random.clone().into(),
            amount: coins(300, "btc"),
        },
        456,
    );
    let msgs = reflect::Message {
        messages: vec![msg],
    };
    let _res = app
        .execute_contract(random, reflect_addr.clone(), &msgs, &[])
        .unwrap();

    // ensure error was written
    let query = reflect::QueryMsg::Reply { id: 456 };
    let res: Reply = app.wrap().query_wasm_smart(&reflect_addr, &query).unwrap();
    assert_eq!(res.id, 456);
    assert!(res.result.is_err());
    // TODO: check error?
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

    let owner = app.api().addr_make("owner");
    let owner2 = app.api().addr_make("owner2");
    let beneficiary = app.api().addr_make("beneficiary");

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
    let owner = Addr::unchecked("owner");
    let beneficiary = Addr::unchecked("beneficiary");
    let init_funds = coins(30, "btc");

    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner, init_funds)
            .unwrap();
    });

    // create a hackatom contract with some funds
    let code_id = app.store_code(hackatom::contract());

    let contract = app
        .instantiate_contract(
            code_id,
            owner.clone(),
            &hackatom::InstantiateMsg {
                beneficiary: beneficiary.as_str().to_owned(),
            },
            &coins(20, "btc"),
            "Hackatom",
            Some(owner.to_string()),
        )
        .unwrap();

    // check admin set properly
    let info = app.contract_data(&contract).unwrap();
    assert_eq!(info.admin, Some(owner.clone()));
    // check beneficiary set properly
    let state: hackatom::InstantiateMsg = app
        .wrap()
        .query_wasm_smart(&contract, &hackatom::QueryMsg::Beneficiary {})
        .unwrap();
    assert_eq!(state.beneficiary, beneficiary.to_string());

    // migrate fails if not admin
    let random = Addr::unchecked("random");
    let migrate_msg = hackatom::MigrateMsg {
        new_guy: random.to_string(),
    };
    app.migrate_contract(beneficiary, contract.clone(), &migrate_msg, code_id)
        .unwrap_err();

    // migrate fails if unregistered code id
    app.migrate_contract(owner.clone(), contract.clone(), &migrate_msg, code_id + 7)
        .unwrap_err();

    // migrate succeeds when the stars align
    app.migrate_contract(owner, contract.clone(), &migrate_msg, code_id)
        .unwrap();

    // check beneficiary updated
    let state: hackatom::InstantiateMsg = app
        .wrap()
        .query_wasm_smart(&contract, &hackatom::QueryMsg::Beneficiary {})
        .unwrap();
    assert_eq!(state.beneficiary, random.to_string());
}

#[test]
fn sent_funds_properly_visible_on_execution() {
    // Testing if funds on contract are properly visible on contract.
    // Hackatom contract is initialized with 10btc. Then, the contract is executed, with
    // additional 20btc. Then beneficiary balance is checked - expected value is 30btc. 10btc
    // would mean that sending tokens with message is not visible for this very message, and
    // 20btc means, that only such just send funds are visible.
    let api = MockApi::default();
    let owner = api.addr_make("owner");
    let beneficiary = api.addr_make("beneficiary");
    let init_funds = coins(30, "btc");

    let mut app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner, init_funds)
            .unwrap();
    });

    let code_id = app.store_code(hackatom::contract());

    let contract = app
        .instantiate_contract(
            code_id,
            owner.clone(),
            &hackatom::InstantiateMsg {
                beneficiary: beneficiary.as_str().to_owned(),
            },
            &coins(10, "btc"),
            "Hackatom",
            None,
        )
        .unwrap();

    app.execute_contract(
        owner.clone(),
        contract.clone(),
        &Empty {},
        &coins(20, "btc"),
    )
    .unwrap();

    // Check balance of all accounts to ensure no tokens where burned or created, and they are
    // in correct places
    assert_eq!(get_balance(&app, &owner), &[]);
    assert_eq!(get_balance(&app, &contract), &[]);
    assert_eq!(get_balance(&app, &beneficiary), coins(30, "btc"));
}

/// Demonstrates that we can mint tokens and send from other accounts
/// via a custom module, as an example of ability to do privileged actions.
mod custom_handler {
    use super::*;
    use crate::{BankSudo, BasicAppBuilder, CosmosRouter};

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
        ) -> AnyResult<AppResponse>
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

        fn sudo<ExecC, QueryC>(
            &self,
            _api: &dyn Api,
            _storage: &mut dyn Storage,
            _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
            _block: &BlockInfo,
            _msg: Self::SudoT,
        ) -> AnyResult<AppResponse>
        where
            ExecC: CustomMsg + DeserializeOwned + 'static,
            QueryC: CustomQuery + DeserializeOwned + 'static,
        {
            bail!("sudo not implemented for CustomHandler")
        }

        fn query(
            &self,
            _api: &dyn Api,
            _storage: &dyn Storage,
            _querier: &dyn Querier,
            _block: &BlockInfo,
            _request: Self::QueryT,
        ) -> AnyResult<Binary> {
            bail!("query not implemented for CustomHandler")
        }
    }

    impl CustomHandler {
        // this is a custom initialization method
        pub fn set_payout(
            &self,
            storage: &mut dyn Storage,
            lottery: Coin,
            pity: Coin,
        ) -> AnyResult<()> {
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
    use cosmwasm_std::to_json_binary;

    use echo::EXECUTE_REPLY_BASE_ID;

    fn make_echo_submsg(
        contract: Addr,
        data: impl Into<Option<&'static str>>,
        sub_msg: Vec<SubMsg>,
        id: u64,
    ) -> SubMsg {
        let data = data.into().map(|s| s.to_owned());
        SubMsg::reply_always(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract.into(),
                msg: to_json_binary(&echo::Message {
                    data,
                    sub_msg,
                    ..echo::Message::default()
                })
                .unwrap(),
                funds: vec![],
            }),
            id,
        )
    }

    fn make_echo_submsg_no_reply(
        contract: Addr,
        data: impl Into<Option<&'static str>>,
        sub_msg: Vec<SubMsg>,
    ) -> SubMsg {
        let data = data.into().map(|s| s.to_owned());
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract.into(),
            msg: to_json_binary(&echo::Message {
                data,
                sub_msg,
                ..echo::Message::default()
            })
            .unwrap(),
            funds: vec![],
        }))
    }

    #[test]
    fn no_submsg() {
        let mut app = App::default();

        let owner = Addr::unchecked("owner");

        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let response = app
            .execute_contract(
                owner,
                contract,
                &echo::Message::<Empty> {
                    data: Some("Data".to_owned()),
                    ..echo::Message::default()
                },
                &[],
            )
            .unwrap();

        assert_eq!(response.data, Some(b"Data".into()));
    }

    #[test]
    fn single_submsg() {
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
                &echo::Message {
                    data: Some("First".to_owned()),
                    sub_msg: vec![make_echo_submsg(
                        contract,
                        "Second",
                        vec![],
                        EXECUTE_REPLY_BASE_ID,
                    )],
                    ..echo::Message::default()
                },
                &[],
            )
            .unwrap();

        assert_eq!(response.data, Some(b"Second".into()));
    }

    #[test]
    fn single_submsg_no_reply() {
        let mut app = App::default();

        let owner = Addr::unchecked("owner");

        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let response = app
            .execute_contract(
                owner,
                contract.clone(),
                &echo::Message {
                    data: Some("First".to_owned()),
                    sub_msg: vec![make_echo_submsg_no_reply(contract, "Second", vec![])],
                    ..echo::Message::default()
                },
                &[],
            )
            .unwrap();

        assert_eq!(response.data, Some(b"First".into()));
    }

    #[test]
    fn single_no_submsg_data() {
        let mut app = App::default();

        let owner = Addr::unchecked("owner");

        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let response = app
            .execute_contract(
                owner,
                contract.clone(),
                &echo::Message {
                    data: Some("First".to_owned()),
                    sub_msg: vec![make_echo_submsg(contract, None, vec![], 1)],
                    ..echo::Message::default()
                },
                &[],
            )
            .unwrap();

        assert_eq!(response.data, Some(b"First".into()));
    }

    #[test]
    fn single_no_top_level_data() {
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
                &echo::Message {
                    sub_msg: vec![make_echo_submsg(
                        contract,
                        "Second",
                        vec![],
                        EXECUTE_REPLY_BASE_ID,
                    )],
                    ..echo::Message::default()
                },
                &[],
            )
            .unwrap();

        assert_eq!(response.data, Some(b"Second".into()));
    }

    #[test]
    fn single_submsg_reply_returns_none() {
        // set personal balance
        let owner = Addr::unchecked("owner");
        let init_funds = coins(100, "tgd");

        let mut app = custom_app::<CustomHelperMsg, Empty, _>(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &owner, init_funds)
                .unwrap();
        });

        // set up reflect contract
        let reflect_id = app.store_code(reflect::contract());

        let reflect_addr = app
            .instantiate_contract(reflect_id, owner.clone(), &Empty {}, &[], "Reflect", None)
            .unwrap();

        // set up echo contract
        let echo_id = app.store_code(echo::custom_contract());

        let echo_addr = app
            .instantiate_contract(echo_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        // reflect will call echo
        // echo will set the data
        // top-level app will not display the data
        let echo_msg = echo::Message::<Empty> {
            data: Some("my echo".into()),
            events: vec![Event::new("echo").add_attribute("called", "true")],
            ..echo::Message::default()
        };
        let reflect_msg = reflect::Message {
            messages: vec![SubMsg::new(WasmMsg::Execute {
                contract_addr: echo_addr.to_string(),
                msg: to_json_binary(&echo_msg).unwrap(),
                funds: vec![],
            })],
        };

        let res = app
            .execute_contract(owner, reflect_addr.clone(), &reflect_msg, &[])
            .unwrap();

        // ensure data is empty
        assert_eq!(res.data, None);
        // ensure expected events
        assert_eq!(res.events.len(), 3, "{:?}", res.events);
        res.assert_event(&Event::new("execute").add_attribute("_contract_address", &reflect_addr));
        res.assert_event(&Event::new("execute").add_attribute("_contract_address", &echo_addr));
        res.assert_event(&Event::new("wasm-echo"));
    }

    #[test]
    fn multiple_submsg() {
        let mut app = App::default();

        let owner = Addr::unchecked("owner");

        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let response = app
            .execute_contract(
                owner,
                contract.clone(),
                &echo::Message {
                    data: Some("Orig".to_owned()),
                    sub_msg: vec![
                        make_echo_submsg(contract.clone(), None, vec![], EXECUTE_REPLY_BASE_ID + 1),
                        make_echo_submsg(
                            contract.clone(),
                            "First",
                            vec![],
                            EXECUTE_REPLY_BASE_ID + 2,
                        ),
                        make_echo_submsg(
                            contract.clone(),
                            "Second",
                            vec![],
                            EXECUTE_REPLY_BASE_ID + 3,
                        ),
                        make_echo_submsg(contract, None, vec![], EXECUTE_REPLY_BASE_ID + 4),
                    ],
                    ..echo::Message::default()
                },
                &[],
            )
            .unwrap();

        assert_eq!(response.data, Some(b"Second".into()));
    }

    #[test]
    fn multiple_submsg_no_reply() {
        let mut app = App::default();

        let owner = Addr::unchecked("owner");

        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let response = app
            .execute_contract(
                owner,
                contract.clone(),
                &echo::Message {
                    data: Some("Orig".to_owned()),
                    sub_msg: vec![
                        make_echo_submsg_no_reply(contract.clone(), None, vec![]),
                        make_echo_submsg_no_reply(contract.clone(), "First", vec![]),
                        make_echo_submsg_no_reply(contract.clone(), "Second", vec![]),
                        make_echo_submsg_no_reply(contract, None, vec![]),
                    ],
                    ..echo::Message::default()
                },
                &[],
            )
            .unwrap();

        assert_eq!(response.data, Some(b"Orig".into()));
    }

    #[test]
    fn multiple_submsg_mixed() {
        let mut app = App::default();

        let owner = Addr::unchecked("owner");

        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let response = app
            .execute_contract(
                owner,
                contract.clone(),
                &echo::Message {
                    sub_msg: vec![
                        make_echo_submsg(contract.clone(), None, vec![], EXECUTE_REPLY_BASE_ID + 1),
                        make_echo_submsg_no_reply(contract.clone(), "Hidden", vec![]),
                        make_echo_submsg(
                            contract.clone(),
                            "Shown",
                            vec![],
                            EXECUTE_REPLY_BASE_ID + 2,
                        ),
                        make_echo_submsg(contract.clone(), None, vec![], EXECUTE_REPLY_BASE_ID + 3),
                        make_echo_submsg_no_reply(contract, "Lost", vec![]),
                    ],
                    ..echo::Message::default()
                },
                &[],
            )
            .unwrap();

        assert_eq!(response.data, Some(b"Shown".into()));
    }

    #[test]
    fn nested_submsg() {
        let mut app = App::default();

        let owner = Addr::unchecked("owner");

        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let response = app
            .execute_contract(
                owner,
                contract.clone(),
                &echo::Message {
                    data: Some("Orig".to_owned()),
                    sub_msg: vec![make_echo_submsg(
                        contract.clone(),
                        None,
                        vec![make_echo_submsg(
                            contract.clone(),
                            "First",
                            vec![make_echo_submsg(
                                contract.clone(),
                                "Second",
                                vec![make_echo_submsg(
                                    contract,
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
                    ..echo::Message::default()
                },
                &[],
            )
            .unwrap();

        assert_eq!(response.data, Some(b"Second".into()));
    }
}

mod response_validation {
    use super::*;
    use crate::error::Error;

    #[test]
    fn empty_attribute_key() {
        let mut app = App::default();

        let owner = Addr::unchecked("owner");

        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let err = app
            .execute_contract(
                owner,
                contract,
                &echo::Message::<Empty> {
                    data: None,
                    attributes: vec![
                        Attribute::new("   ", "value"),
                        Attribute::new("proper", "proper_val"),
                    ],
                    ..echo::Message::default()
                },
                &[],
            )
            .unwrap_err();

        assert_eq!(Error::empty_attribute_key("value"), err.downcast().unwrap(),);
    }

    #[test]
    fn empty_attribute_value() {
        let mut app = App::default();

        let owner = Addr::unchecked("owner");

        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let err = app
            .execute_contract(
                owner,
                contract,
                &echo::Message::<Empty> {
                    data: None,
                    attributes: vec![
                        Attribute::new("key", "   "),
                        Attribute::new("proper", "proper_val"),
                    ],
                    ..echo::Message::default()
                },
                &[],
            )
            .unwrap_err();

        assert_eq!(Error::empty_attribute_value("key"), err.downcast().unwrap());
    }

    #[test]
    fn empty_event_attribute_key() {
        let mut app = App::default();

        let owner = Addr::unchecked("owner");

        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let err = app
            .execute_contract(
                owner,
                contract,
                &echo::Message::<Empty> {
                    data: None,
                    events: vec![Event::new("event")
                        .add_attribute("   ", "value")
                        .add_attribute("proper", "proper_val")],
                    ..echo::Message::default()
                },
                &[],
            )
            .unwrap_err();

        assert_eq!(Error::empty_attribute_key("value"), err.downcast().unwrap());
    }

    #[test]
    fn empty_event_attribute_value() {
        let mut app = App::default();

        let owner = Addr::unchecked("owner");

        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let err = app
            .execute_contract(
                owner,
                contract,
                &echo::Message::<Empty> {
                    data: None,
                    events: vec![Event::new("event")
                        .add_attribute("key", "   ")
                        .add_attribute("proper", "proper_val")],
                    ..echo::Message::default()
                },
                &[],
            )
            .unwrap_err();

        assert_eq!(Error::empty_attribute_value("key"), err.downcast().unwrap());
    }

    #[test]
    fn too_short_event_type() {
        let mut app = App::default();

        let owner = Addr::unchecked("owner");

        let code_id = app.store_code(echo::contract());

        let contract = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "Echo", None)
            .unwrap();

        let err = app
            .execute_contract(
                owner,
                contract,
                &echo::Message::<Empty> {
                    data: None,
                    events: vec![Event::new(" e "), Event::new("event")],
                    ..echo::Message::default()
                },
                &[],
            )
            .unwrap_err();

        assert_eq!(Error::event_type_too_short("e"), err.downcast().unwrap());
    }
}

mod contract_instantiation {

    #[test]
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
            salt: salt.clone().into(),
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
    fn query_non_existing_code_info() {
        use super::*;
        let app = App::default();
        assert_eq!(
            "Generic error: Querier contract error: code id: invalid",
            app.wrap().query_wasm_code_info(0).unwrap_err().to_string()
        );
        assert_eq!(
            "Generic error: Querier contract error: code id 1: no such code",
            app.wrap().query_wasm_code_info(1).unwrap_err().to_string()
        );
    }
}

mod custom_messages {
    use super::*;

    #[test]
    fn triggering_custom_msg() {
        let custom_handler = CachingCustomHandler::<CustomHelperMsg, Empty>::new();
        let custom_handler_state = custom_handler.state();

        let mut app = AppBuilder::new_custom()
            .with_custom(custom_handler)
            .build(no_init);

        let sender = app.api().addr_make("sender");
        let owner = app.api().addr_make("owner");

        let contract_id = app.store_code(echo::custom_contract());

        let contract = app
            .instantiate_contract(contract_id, owner, &Empty {}, &[], "Echo", None)
            .unwrap();

        app.execute_contract(
            sender,
            contract,
            &echo::Message {
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
        // set personal balance
        let owner = Addr::unchecked("owner");
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
            .query_wasm_smart(&parsed.contract_address, &reflect::QueryMsg::Count {})
            .unwrap();
        assert_eq!(count.count, 0);
    }

    #[test]
    fn instantiate_with_data_works() {
        let owner = Addr::unchecked("owner");
        let mut app = BasicApp::new(no_init);

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
        let owner = Addr::unchecked("owner");
        let mut app = BasicApp::new(no_init);

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
        let msg = echo::Message::<Empty> {
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
        let owner = Addr::unchecked("owner");
        let mut app = BasicApp::new(no_init);

        // set up reflect contract
        let code_id = app.store_code(echo::contract());

        let echo_addr = app
            .instantiate_contract(code_id, owner.clone(), &Empty {}, &[], "label", None)
            .unwrap();

        // ensure the execute has the same wrapper as it should
        let msg = echo::Message::<Empty> {
            data: Some("hello".into()),
            ..echo::Message::default()
        };
        // execute_contract now decodes a protobuf wrapper, so we get the top-level response
        let exec_res = app.execute_contract(owner, echo_addr, &msg, &[]).unwrap();
        assert_eq!(exec_res.data, Some(Binary::from(b"hello")));
    }
}

mod errors {
    use super::*;
    use cosmwasm_std::to_json_binary;

    #[test]
    fn simple_instantiation() {
        let owner = Addr::unchecked("owner");
        let mut app = App::default();

        // set up contract
        let code_id = app.store_code(error::contract(false));

        let msg = Empty {};
        let err = app
            .instantiate_contract(code_id, owner, &msg, &[], "error", None)
            .unwrap_err();

        // we should be able to retrieve the original error by downcasting
        let source: &StdError = err.downcast_ref().unwrap();
        if let StdError::GenericErr { msg, .. } = source {
            assert_eq!(msg, "Init failed");
        } else {
            panic!("wrong StdError variant");
        }

        // We're expecting exactly 2 nested error types
        // (the original error, WasmMsg context)
        assert_eq!(err.chain().count(), 2);
    }

    #[test]
    fn simple_call() {
        let owner = Addr::unchecked("owner");
        let mut app = App::default();

        // set up contract
        let code_id = app.store_code(error::contract(true));

        let msg = Empty {};
        let contract_addr = app
            .instantiate_contract(code_id, owner, &msg, &[], "error", None)
            .unwrap();

        // execute should error
        let err = app
            .execute_contract(Addr::unchecked("random"), contract_addr, &msg, &[])
            .unwrap_err();

        // we should be able to retrieve the original error by downcasting
        let source: &StdError = err.downcast_ref().unwrap();
        if let StdError::GenericErr { msg, .. } = source {
            assert_eq!(msg, "Handle failed");
        } else {
            panic!("wrong StdError variant");
        }

        // We're expecting exactly 2 nested error types
        // (the original error, WasmMsg context)
        assert_eq!(err.chain().count(), 2);
    }

    #[test]
    fn nested_call() {
        let owner = Addr::unchecked("owner");
        let mut app = App::default();

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
            .execute_contract(Addr::unchecked("random"), caller_addr, &msg, &[])
            .unwrap_err();

        // we can get the original error by downcasting
        let source: &StdError = err.downcast_ref().unwrap();
        if let StdError::GenericErr { msg, .. } = source {
            assert_eq!(msg, "Handle failed");
        } else {
            panic!("wrong StdError variant");
        }

        // We're expecting exactly 3 nested error types
        // (the original error, 2 WasmMsg contexts)
        assert_eq!(err.chain().count(), 3);
    }

    #[test]
    fn double_nested_call() {
        let owner = Addr::unchecked("owner");
        let mut app = App::default();

        let error_code_id = app.store_code(error::contract(true));
        let caller_code_id = app.store_code(caller::contract());

        // set up contract_helpers
        let msg = Empty {};
        let caller_addr1 = app
            .instantiate_contract(caller_code_id, owner.clone(), &msg, &[], "caller", None)
            .unwrap();
        let caller_addr2 = app
            .instantiate_contract(caller_code_id, owner.clone(), &msg, &[], "caller", None)
            .unwrap();
        let error_addr = app
            .instantiate_contract(error_code_id, owner, &msg, &[], "error", None)
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
            .execute_contract(Addr::unchecked("random"), caller_addr1, &msg, &[])
            .unwrap_err();

        // uncomment to have the test fail and see how the error stringifies
        // panic!("{:?}", err);

        // we can get the original error by downcasting
        let source: &StdError = err.downcast_ref().unwrap();
        if let StdError::GenericErr { msg, .. } = source {
            assert_eq!(msg, "Handle failed");
        } else {
            panic!("wrong StdError variant");
        }

        // We're expecting exactly 4 nested error types
        // (the original error, 3 WasmMsg contexts)
        assert_eq!(err.chain().count(), 4);
    }
}
