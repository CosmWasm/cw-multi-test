use cosmwasm_std::{coin, Addr};
use cw_multi_test::{App, IntoAddr};
use cw_storage_plus::Map;
use cw_utils::NativeBalance;
use std::ops::{Deref, DerefMut};

const NAMESPACE_CENTRAL_BANK: &[u8] = b"central-bank";
const NAMESPACE_NATIONAL_BANK: &[u8] = b"national-bank";
const NAMESPACE_LOCAL_BANK: &[u8] = b"local-bank";
const NAMESPACES: &[&[u8]] = &[
    NAMESPACE_NATIONAL_BANK,
    NAMESPACE_CENTRAL_BANK,
    NAMESPACE_LOCAL_BANK,
];
const BALANCES: Map<&Addr, NativeBalance> = Map::new("balances");

#[test]
fn multilevel_storage_should_work() {
    // prepare balance owner
    let owner_addr = "owner".into_addr();
    // create the blockchain
    let mut app = App::default();
    {
        // get the mutable prefixed, multilevel storage for banks
        let mut storage_mut = app.prefixed_multilevel_storage_mut(NAMESPACES);
        // set balances manually
        let mut balance = NativeBalance(vec![coin(111, "BTC"), coin(222, "ETH")]);
        balance.normalize();
        BALANCES
            .save(storage_mut.deref_mut(), &owner_addr, &balance)
            .unwrap();
    }
    {
        // get the read-only prefixed, multilevel storage for banks
        let storage = app.prefixed_multilevel_storage(NAMESPACES);
        // read balances manually
        let balances = BALANCES.load(storage.deref(), &owner_addr).unwrap();
        assert_eq!("BTC111ETH222", balances.to_string());
    }
}
