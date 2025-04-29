use cosmwasm_std::{
    coin, from_json, testing::MockApi, to_json_binary, AllBalanceResponse, BankQuery, CosmosMsg,
    Empty, IbcMsg, IbcOrder, IbcTimeout, IbcTimeoutBlock, Querier, QueryRequest,
};
use cw_multi_test::{
    ibc::{
        relayer::{create_channel, create_connection, relay_packets_in_tx, ChannelCreationResult},
        IbcSimpleModule,
    },
    no_init, AppBuilder, Executor,
};

/// In this module, we are testing the bank module ibc capabilities
/// We try in the implementation to stay simple but as close as the real deal as possible

#[test]
fn simple_transfer() -> anyhow::Result<()> {
    let funds = coin(100_000, "ufund");

    let mut app1 = AppBuilder::default()
        .with_api(MockApi::default().with_prefix("src"))
        .with_ibc(IbcSimpleModule)
        .build(no_init);

    let mut app2 = AppBuilder::default()
        .with_api(MockApi::default().with_prefix("dst"))
        .with_ibc(IbcSimpleModule)
        .build(no_init);

    // We add a start balance for the owner
    let fund_owner = app1.api().addr_make("owner");
    let fund_recipient = app2.api().addr_make("recipient");
    app1.init_modules(|router, _api, storage| {
        router
            .bank
            .init_balance(storage, &fund_owner, vec![funds.clone()])
            .unwrap();
    });

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
        port1,
        port2,
        "ics20-1".to_string(),
        IbcOrder::Ordered,
    )?;

    // We send an IBC transfer Cosmos Msg on app 1
    let send_response = app1.execute(
        fund_owner.clone(),
        CosmosMsg::Ibc(IbcMsg::Transfer {
            channel_id: src_channel,
            to_address: fund_recipient.to_string(),
            amount: funds.clone(),
            timeout: IbcTimeout::with_block(IbcTimeoutBlock {
                revision: 1,
                height: app2.block_info().height + 1,
            }),
            memo: None,
        }),
    )?;

    // We are relaying all packets found in the transaction.
    relay_packets_in_tx(&mut app1, &mut app2, send_response)?;

    // We make sure the balance of the recipient has changed.
    let balances = app2
        .raw_query(
            to_json_binary(&QueryRequest::<Empty>::Bank(
                #[allow(deprecated)]
                BankQuery::AllBalances {
                    address: fund_recipient.to_string(),
                },
            ))?
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
            to_json_binary(&QueryRequest::<Empty>::Bank(
                #[allow(deprecated)]
                BankQuery::AllBalances {
                    address: fund_owner.to_string(),
                },
            ))?
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
    let funds = coin(100_000, "ufund");

    let port1 = "transfer".to_string();
    let port2 = "transfer".to_string();

    let mut app1 = AppBuilder::default()
        .with_api(MockApi::default().with_prefix("src"))
        .with_ibc(IbcSimpleModule)
        .build(no_init);

    let mut app2 = AppBuilder::default()
        .with_api(MockApi::default().with_prefix("dst"))
        .with_ibc(IbcSimpleModule)
        .build(no_init);

    // We add a start balance for the owner
    let fund_owner = app1.api().addr_make("owner");
    let fund_recipient = app2.api().addr_make("recipient");
    app1.init_modules(|router, _api, storage| {
        router
            .bank
            .init_balance(storage, &fund_owner, vec![funds.clone()])
            .unwrap();
    });

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
        port1,
        port2,
        "ics20-1".to_string(),
        IbcOrder::Ordered,
    )?;

    // We send an IBC transfer Cosmos Msg on app 1
    let send_response = app1.execute(
        fund_owner.clone(),
        CosmosMsg::Ibc(IbcMsg::Transfer {
            channel_id: src_channel,
            to_address: fund_recipient.to_string(),
            amount: funds.clone(),
            timeout: IbcTimeout::with_block(IbcTimeoutBlock {
                revision: 1,
                height: app2.block_info().height + 1,
            }),
            memo: None,
        }),
    )?;

    // We are relaying all packets found in the transaction.
    relay_packets_in_tx(&mut app1, &mut app2, send_response)?;

    // TODO:  We can't verify the funds are locked because the IBC_LOCK_MODULE_ADDRESS is not valid.
    // let balances = app1
    //     .raw_query(
    //         to_json_binary(&QueryRequest::<Empty>::Bank(BankQuery::AllBalances {
    //             address: IBC_LOCK_MODULE_ADDRESS.to_string(),
    //         }))?
    //         .as_slice(),
    //     )
    //     .into_result()?
    //     .unwrap();
    // let balances: AllBalanceResponse = from_json(balances)?;
    // assert_eq!(balances.amount.len(), 1);
    // assert_eq!(balances.amount[0].amount, funds.amount);
    // assert_eq!(balances.amount[0].denom, funds.denom);

    let chain2_funds = coin(
        funds.amount.u128(),
        format!("ibc/{}/{}", dst_channel, funds.denom),
    );
    // We send an IBC transfer back from app2
    let send_back_response = app2.execute(
        fund_recipient.clone(),
        CosmosMsg::Ibc(IbcMsg::Transfer {
            channel_id: dst_channel,
            to_address: fund_owner.to_string(),
            amount: chain2_funds,
            timeout: IbcTimeout::with_block(IbcTimeoutBlock {
                revision: 1,
                height: app2.block_info().height + 100,
            }),
            memo: None,
        }),
    )?;

    // We are relaying all packets found in the transaction.
    relay_packets_in_tx(&mut app2, &mut app1, send_back_response)?;

    // We make sure the balance of the recipient has changed
    let balances = app2
        .raw_query(
            to_json_binary(&QueryRequest::<Empty>::Bank(
                #[allow(deprecated)]
                BankQuery::AllBalances {
                    address: fund_recipient.to_string(),
                },
            ))?
            .as_slice(),
        )
        .into_result()?
        .unwrap();
    let balances: AllBalanceResponse = from_json(balances)?;
    assert!(balances.amount.is_empty());

    // We make sure the balance of the sender has changed as well.
    let balances = app1
        .raw_query(
            to_json_binary(&QueryRequest::<Empty>::Bank(
                #[allow(deprecated)]
                BankQuery::AllBalances {
                    address: fund_owner.to_string(),
                },
            ))?
            .as_slice(),
        )
        .into_result()?
        .unwrap();
    let balances: AllBalanceResponse = from_json(balances)?;

    // The owner has back exactly what they need.
    assert_eq!(balances.amount.len(), 1);
    assert_eq!(balances.amount[0].amount, funds.amount);
    assert_eq!(balances.amount[0].denom, funds.denom);

    // TODO:  We can't verify the funds are locked because the IBC_LOCK_MODULE_ADDRESS is not valid.
    // // Same for ibc lock address
    // let balances = app1
    //     .raw_query(
    //         to_json_binary(&QueryRequest::<Empty>::Bank(BankQuery::AllBalances {
    //             address: IBC_LOCK_MODULE_ADDRESS.to_string(),
    //         }))?
    //         .as_slice(),
    //     )
    //     .into_result()?
    //     .unwrap();
    // let balances: AllBalanceResponse = from_json(balances)?;
    // assert_eq!(balances.amount.len(), 0);

    Ok(())
}
