use cosmwasm_std::{
    coin, from_json, to_json_binary, Addr, AllBalanceResponse, BankQuery, CosmosMsg, Empty, IbcMsg,
    IbcOrder, IbcTimeout, IbcTimeoutBlock, Querier, QueryRequest,
};

use crate::{
    bank::IBC_LOCK_MODULE_ADDRESS,
    ibc::{
        relayer::{create_channel, create_connection, relay_packets_in_tx, ChannelCreationResult},
        simple_ibc::IbcSimpleModule,
        test::init,
    },
    AppBuilder, Executor,
};

/// In this module, we are testing the bank module ibc capabilities
/// We try in the implementation to stay simple but as close as the real deal as possible

#[test]
fn simple_transfer() -> anyhow::Result<()> {
    init();

    let funds = coin(100_000, "ufund");
    let fund_owner = "owner";
    let fund_recipient = "recipient";

    // We mint some funds to the owner
    let mut app1 = AppBuilder::default()
        .with_ibc(IbcSimpleModule)
        .build(|router, api, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &api.addr_validate(fund_owner).unwrap(),
                    vec![funds.clone()],
                )
                .unwrap();
        });
    let mut app2 = AppBuilder::default()
        .with_ibc(IbcSimpleModule)
        .build(|_, _, _| {});

    let port1 = "transfer".to_string();
    let port2 = "transfer".to_string();

    let (src_connection_id, _) = create_connection(&mut app1, &mut app2)?;

    // We start by creating channels
    let ChannelCreationResult {
        src_channel,
        dst_channel,
        ..
    } = create_channel(
        &mut app1,
        &mut app2,
        src_connection_id,
        port1.clone(),
        port2,
        "ics20-1".to_string(),
        IbcOrder::Ordered,
    )?;

    // We send an IBC transfer Cosmos Msg on app 1
    let send_response = app1.execute(
        Addr::unchecked(fund_owner),
        CosmosMsg::Ibc(IbcMsg::Transfer {
            channel_id: src_channel,
            to_address: fund_recipient.to_string(),
            amount: funds.clone(),
            timeout: IbcTimeout::with_block(IbcTimeoutBlock {
                revision: 1,
                height: app1.block_info().height,
            }),
        }),
    )?;

    // We relaying all packets found in the transaction
    relay_packets_in_tx(&mut app1, &mut app2, send_response)?;

    // We make sure the balance of the reciepient has changed
    let balances = app2
        .raw_query(
            to_json_binary(&QueryRequest::<Empty>::Bank(BankQuery::AllBalances {
                address: fund_recipient.to_string(),
            }))?
            .as_slice(),
        )
        .into_result()?
        .unwrap();
    let balances: AllBalanceResponse = from_json(balances)?;

    // The recipient has received exactly what they needs
    assert_eq!(balances.amount.len(), 1);
    assert_eq!(balances.amount[0].amount, funds.amount);
    assert_eq!(
        balances.amount[0].denom,
        format!("ibc/{}/{}", dst_channel, funds.denom)
    );

    // We make sure the balance of the sender has changed as well
    let balances = app1
        .raw_query(
            to_json_binary(&QueryRequest::<Empty>::Bank(BankQuery::AllBalances {
                address: fund_owner.to_string(),
            }))?
            .as_slice(),
        )
        .into_result()?
        .unwrap();
    let balances: AllBalanceResponse = from_json(balances)?;
    assert!(balances.amount.is_empty());

    Ok(())
}

#[test]
fn transfer_and_back() -> anyhow::Result<()> {
    init();

    let funds = coin(100_000, "ufund");
    let fund_owner = "owner";
    let fund_recipient = "recipient";

    let port1 = "transfer".to_string();
    let port2 = "transfer".to_string();

    // We mint some funds to the owner
    let mut app1 = AppBuilder::default()
        .with_ibc(IbcSimpleModule)
        .build(|router, api, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &api.addr_validate(fund_owner).unwrap(),
                    vec![funds.clone()],
                )
                .unwrap();
        });
    let mut app2 = AppBuilder::default()
        .with_ibc(IbcSimpleModule)
        .build(|_, _, _| {});

    let (src_connection_id, _) = create_connection(&mut app1, &mut app2)?;

    // We start by creating channels
    let ChannelCreationResult {
        src_channel,
        dst_channel,
        ..
    } = create_channel(
        &mut app1,
        &mut app2,
        src_connection_id,
        port1.clone(),
        port2,
        "ics20-1".to_string(),
        IbcOrder::Ordered,
    )?;

    // We send an IBC transfer Cosmos Msg on app 1
    let send_response = app1.execute(
        Addr::unchecked(fund_owner),
        CosmosMsg::Ibc(IbcMsg::Transfer {
            channel_id: src_channel,
            to_address: fund_recipient.to_string(),
            amount: funds.clone(),
            timeout: IbcTimeout::with_block(IbcTimeoutBlock {
                revision: 1,
                height: app1.block_info().height,
            }),
        }),
    )?;

    // We relaying all packets found in the transaction
    relay_packets_in_tx(&mut app1, &mut app2, send_response)?;

    // We verify the funds are locked
    let balances = app1
        .raw_query(
            to_json_binary(&QueryRequest::<Empty>::Bank(BankQuery::AllBalances {
                address: IBC_LOCK_MODULE_ADDRESS.to_string(),
            }))?
            .as_slice(),
        )
        .into_result()?
        .unwrap();
    let balances: AllBalanceResponse = from_json(balances)?;
    assert_eq!(balances.amount.len(), 1);
    assert_eq!(balances.amount[0].amount, funds.amount);
    assert_eq!(balances.amount[0].denom, funds.denom);

    let chain2_funds = coin(
        funds.amount.u128(),
        format!("ibc/{}/{}", dst_channel, funds.denom),
    );
    // We send an IBC transfer back from app2
    let send_back_response = app2.execute(
        Addr::unchecked(fund_recipient),
        CosmosMsg::Ibc(IbcMsg::Transfer {
            channel_id: dst_channel,
            to_address: fund_owner.to_string(),
            amount: chain2_funds.clone(),
            timeout: IbcTimeout::with_block(IbcTimeoutBlock {
                revision: 1,
                height: app2.block_info().height + 100,
            }),
        }),
    )?;

    // We relaying all packets found in the transaction
    relay_packets_in_tx(&mut app2, &mut app1, send_back_response)?;

    // We make sure the balance of the reciepient has changed
    let balances = app2
        .raw_query(
            to_json_binary(&QueryRequest::<Empty>::Bank(BankQuery::AllBalances {
                address: fund_recipient.to_string(),
            }))?
            .as_slice(),
        )
        .into_result()?
        .unwrap();
    let balances: AllBalanceResponse = from_json(balances)?;
    assert!(balances.amount.is_empty());

    // We make sure the balance of the sender has changed as well
    let balances = app1
        .raw_query(
            to_json_binary(&QueryRequest::<Empty>::Bank(BankQuery::AllBalances {
                address: fund_owner.to_string(),
            }))?
            .as_slice(),
        )
        .into_result()?
        .unwrap();
    let balances: AllBalanceResponse = from_json(balances)?;

    // The owner has back exactly what they need
    assert_eq!(balances.amount.len(), 1);
    assert_eq!(balances.amount[0].amount, funds.amount);
    assert_eq!(balances.amount[0].denom, funds.denom);

    // Same for ibc lock address
    let balances = app1
        .raw_query(
            to_json_binary(&QueryRequest::<Empty>::Bank(BankQuery::AllBalances {
                address: IBC_LOCK_MODULE_ADDRESS.to_string(),
            }))?
            .as_slice(),
        )
        .into_result()?
        .unwrap();
    let balances: AllBalanceResponse = from_json(balances)?;
    assert_eq!(balances.amount.len(), 0);

    Ok(())
}
