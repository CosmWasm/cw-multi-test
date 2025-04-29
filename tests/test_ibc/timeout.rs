use cosmwasm_std::{
    coin, from_json, testing::MockApi, to_json_binary, Addr, AllBalanceResponse, BankQuery,
    CosmosMsg, Empty, IbcMsg, IbcOrder, IbcTimeout, IbcTimeoutBlock, Querier, QueryRequest,
};
use cw_multi_test::{
    ibc::{
        events::TIMEOUT_RECEIVE_PACKET_EVENT,
        relayer::{
            create_channel, create_connection, has_event, relay_packets_in_tx,
            ChannelCreationResult, RelayingResult,
        },
        types::{ChannelInfo, MockIbcQuery},
        IbcSimpleModule,
    },
    no_init, AppBuilder, Executor,
};

#[test]
fn simple_transfer_timeout() -> anyhow::Result<()> {
    let funds = coin(100_000, "ufund");

    let mut app1 = AppBuilder::default()
        .with_api(MockApi::default().with_prefix("src"))
        .with_ibc(IbcSimpleModule)
        .build(no_init);

    let mut app2 = AppBuilder::default()
        .with_api(MockApi::default().with_prefix("dst"))
        .with_ibc(IbcSimpleModule)
        .build(no_init);

    // We add a start balance for the owner.
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

    // We start by creating channels.
    let ChannelCreationResult { src_channel, .. } = create_channel(
        &mut app1,
        &mut app2,
        src_connection_id,
        port1,
        port2,
        "ics20-1".to_string(),
        IbcOrder::Ordered,
    )?;

    // We send an IBC transfer Cosmos Msg on app 1.
    let send_response = app1.execute(
        fund_owner.clone(),
        CosmosMsg::Ibc(IbcMsg::Transfer {
            channel_id: src_channel,
            to_address: fund_recipient.to_string(),
            amount: funds.clone(),
            timeout: IbcTimeout::with_block(IbcTimeoutBlock {
                revision: 1,
                height: app2.block_info().height, // this will have the effect of a timeout when relaying the packets
            }),
            memo: None,
        }),
    )?;

    // We assert the sender balance is empty !

    // We make sure the balance of the sender hasn't changed in the process.
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

    // We are relaying all packets found in the transaction.
    let resp = relay_packets_in_tx(&mut app1, &mut app2, send_response)?;

    // We make sure the response contains a timeout.
    assert_eq!(resp.len(), 1);
    if let RelayingResult::Acknowledgement { .. } = resp[0].result {
        panic!("Expected a timeout");
    }
    assert!(has_event(&resp[0].receive_tx, TIMEOUT_RECEIVE_PACKET_EVENT));

    // We make sure the balance of the recipient has not changed.
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

    // The recipient has exactly no balance, because it has timed out.
    assert_eq!(balances.amount.len(), 0);

    // We make sure the balance of the sender hasn't changed in the process.
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
    println!("{:?}", balances);
    assert_eq!(balances.amount.len(), 1);
    assert_eq!(balances.amount[0].amount, funds.amount);
    assert_eq!(balances.amount[0].denom, funds.denom);
    Ok(())
}

#[test]
fn simple_transfer_timeout_closes_channel() -> anyhow::Result<()> {
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
        port1.clone(),
        port2.clone(),
        "ics20-1".to_string(),
        IbcOrder::Ordered,
    )?;

    // We send an IBC transfer Cosmos Msg on app 1.
    let send_response = app1.execute(
        Addr::unchecked(fund_owner),
        CosmosMsg::Ibc(IbcMsg::Transfer {
            channel_id: src_channel.clone(),
            to_address: fund_recipient.to_string(),
            amount: funds.clone(),
            timeout: IbcTimeout::with_block(IbcTimeoutBlock {
                revision: 1,
                height: app2.block_info().height, // this will have the effect of a timeout when relaying the packets
            }),
            memo: None,
        }),
    )?;

    // We make sure the channel is open.
    let channel_info: ChannelInfo = from_json(app1.ibc_query(MockIbcQuery::ChannelInfo {
        port_id: port1.clone(),
        channel_id: src_channel.clone(),
    })?)?;
    assert!(channel_info.open);
    // We make sure the channel is open.
    let channel_info: ChannelInfo = from_json(app2.ibc_query(MockIbcQuery::ChannelInfo {
        port_id: port2.clone(),
        channel_id: dst_channel.clone(),
    })?)?;
    assert!(channel_info.open);

    // We are relaying all packets found in the transaction.
    let resp = relay_packets_in_tx(&mut app1, &mut app2, send_response)?;

    // We make sure the response contains a timeout.
    assert_eq!(resp.len(), 1);
    match resp[0].result.clone() {
        RelayingResult::Acknowledgement { .. } => panic!("Expected a timeout"),
        RelayingResult::Timeout {
            close_channel_confirm,
            ..
        } => {
            // We make sure the confirmation of close transaction was executed.
            assert!(close_channel_confirm.is_some())
        }
    }

    // We make sure the channel is closed.
    let channel_info: ChannelInfo = from_json(app1.ibc_query(MockIbcQuery::ChannelInfo {
        port_id: port1,
        channel_id: src_channel,
    })?)?;
    assert!(!channel_info.open);
    // We make sure the channel is closed.
    let channel_info: ChannelInfo = from_json(app2.ibc_query(MockIbcQuery::ChannelInfo {
        port_id: port2,
        channel_id: dst_channel,
    })?)?;
    assert!(!channel_info.open);

    Ok(())
}
