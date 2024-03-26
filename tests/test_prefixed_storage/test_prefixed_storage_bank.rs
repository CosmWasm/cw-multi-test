use cosmwasm_std::{coin, Addr};
use cw_multi_test::{App, IntoAddr};
use cw_storage_plus::Map;
use cw_utils::NativeBalance;
use std::ops::{Deref, DerefMut};

const NAMESPACE_BANK: &[u8] = b"bank";
const BALANCES: Map<&Addr, NativeBalance> = Map::new("balances");

#[test]
fn reading_bank_storage_should_work() {
    // prepare balance owner
    let owner_addr = "owner".into_addr();

    // set balances
    let init_funds = vec![coin(1, "BTC"), coin(2, "ETH")];
    let app = App::new(|router, _, storage| {
        router
            .bank
            .init_balance(storage, &owner_addr, init_funds)
            .unwrap();
    });

    // get the read-only prefixed storage for bank
    let storage = app.prefixed_storage(NAMESPACE_BANK);
    let balances = BALANCES.load(storage.deref(), &owner_addr).unwrap();
    assert_eq!("BTC1ETH2", balances.to_string());
}

#[test]
fn writing_bank_storage_should_work() {
    // prepare balance owner
    let owner_addr = "owner".into_addr();

    let mut app = App::default();
    // get the mutable prefixed storage for bank
    let mut storage = app.prefixed_storage_mut(NAMESPACE_BANK);

    // set balances manually
    let mut balance = NativeBalance(vec![coin(3, "BTC"), coin(4, "ETH")]);
    balance.normalize();
    BALANCES
        .save(storage.deref_mut(), &owner_addr, &balance)
        .unwrap();

    // check balances
    let balances = BALANCES.load(storage.deref(), &owner_addr).unwrap();
    assert_eq!("BTC3ETH4", balances.to_string());
}
