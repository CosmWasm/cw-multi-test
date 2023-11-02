use cosmwasm_std::{
    from_json, to_json_binary, ChannelResponse, Empty, IbcChannel, IbcEndpoint, IbcOrder, IbcQuery,
    Querier, QueryRequest,
};

use crate::{
    ibc::relayer::{create_connection, ChannelCreationResult},
    AppBuilder,
};

use super::{relayer::create_channel, simple_ibc::IbcSimpleModule};

mod bank;
mod polytone;

fn init() {
    //let _ = env_logger::builder().is_test(true).try_init();
    env_logger::init();
}

#[test]
fn channel_creation() -> anyhow::Result<()> {
    init();
    // Here we want to create a channel between 2 bank modules to make sure that we are able to create a channel correctly
    // This is a tracking test for all channel creation
    let mut app1 = AppBuilder::default()
        .with_ibc(IbcSimpleModule)
        .build(|_, _, _| {});
    let mut app2 = AppBuilder::default()
        .with_ibc(IbcSimpleModule)
        .build(|_, _, _| {});

    app1.update_block(|block| block.chain_id = "mock_app_1".to_string());
    app2.update_block(|block| block.chain_id = "mock_app_2".to_string());

    let src_port = "transfer".to_string();
    let dst_port = "transfer".to_string();
    let order = IbcOrder::Unordered;
    let version = "ics20-1".to_string();

    let (src_connection_id, _) = create_connection(&mut app1, &mut app2)?;

    let ChannelCreationResult {
        src_channel,
        dst_channel,
        ..
    } = create_channel(
        &mut app1,
        &mut app2,
        src_connection_id,
        src_port.clone(),
        dst_port.clone(),
        version.clone(),
        order.clone(),
    )?;

    let channel_query = app1
        .raw_query(
            to_json_binary(&QueryRequest::<Empty>::Ibc(IbcQuery::Channel {
                channel_id: src_channel.clone(),
                port_id: Some(src_port.clone()),
            }))?
            .as_slice(),
        )
        .into_result()?
        .unwrap();

    let channel: ChannelResponse = from_json(channel_query)?;

    assert_eq!(
        channel,
        ChannelResponse {
            channel: Some(IbcChannel::new(
                IbcEndpoint {
                    port_id: src_port.clone(),
                    channel_id: src_channel.clone()
                },
                IbcEndpoint {
                    port_id: dst_port.clone(),
                    channel_id: dst_channel.clone()
                },
                order.clone(),
                version.clone(),
                "connection-0"
            ))
        }
    );

    let channel_query = app2
        .raw_query(
            to_json_binary(&QueryRequest::<Empty>::Ibc(IbcQuery::Channel {
                channel_id: dst_channel.clone(),
                port_id: Some(dst_port.clone()),
            }))?
            .as_slice(),
        )
        .into_result()?
        .unwrap();

    let channel: ChannelResponse = from_json(channel_query)?;

    assert_eq!(
        channel,
        ChannelResponse {
            channel: Some(IbcChannel::new(
                IbcEndpoint {
                    port_id: dst_port.clone(),
                    channel_id: dst_channel.clone()
                },
                IbcEndpoint {
                    port_id: src_port.clone(),
                    channel_id: src_channel.clone()
                },
                order,
                version,
                "connection-0"
            ))
        }
    );

    Ok(())
}

#[test]
fn channel_unknown_port() -> anyhow::Result<()> {
    init();
    // Here we want to create a channel between 2 bank modules to make sure that we are able to create a channel correctly
    // This is a tracking test for all channel creation

    let mut app1 = AppBuilder::default()
        .with_ibc(IbcSimpleModule)
        .build(|_, _, _| {});
    let mut app2 = AppBuilder::default()
        .with_ibc(IbcSimpleModule)
        .build(|_, _, _| {});

    let port1 = "other-bad-port".to_string();
    let port2 = "bad-port".to_string();

    let (src_connection_id, _) = create_connection(&mut app1, &mut app2)?;

    create_channel(
        &mut app1,
        &mut app2,
        src_connection_id,
        port1,
        port2,
        "ics20-1".to_string(),
        IbcOrder::Ordered,
    )
    .unwrap_err();

    Ok(())
}
