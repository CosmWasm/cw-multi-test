use anyhow::{anyhow, bail};
use cosmwasm_std::{
    ensure_eq, to_json_binary, Addr, BankMsg, Binary, ChannelResponse, Coin, Event,
    IbcAcknowledgement, IbcChannel, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcEndpoint, IbcMsg,
    IbcOrder, IbcPacket, IbcPacketAckMsg, IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcQuery,
    IbcTimeout, IbcTimeoutBlock, ListChannelsResponse, Order, Storage,
};
use cw20_ics20::ibc::Ics20Packet;

use crate::{
    app::IbcRouterMsg,
    bank::{optional_unwrap_ibc_denom, IBC_LOCK_MODULE_ADDRESS},
    ibc::types::Connection,
    prefixed_storage::{prefixed, prefixed_read},
    transactions::transactional,
    AppResponse, Ibc, Module,
};
use anyhow::Result as AnyResult;

#[derive(Default)]
pub struct IbcSimpleModule;

use super::{
    state::{
        ibc_connections, load_port_info, ACK_PACKET_MAP, CHANNEL_HANDSHAKE_INFO, CHANNEL_INFO,
        NAMESPACE_IBC, PORT_INFO, RECEIVE_PACKET_MAP, SEND_PACKET_MAP, TIMEOUT_PACKET_MAP,
    },
    types::{
        ChannelHandshakeInfo, ChannelHandshakeState, ChannelInfo, IbcPacketAck, IbcPacketData,
        IbcPacketRelayingMsg, IbcResponse, MockIbcPort, MockIbcQuery,
    },
};

pub const RELAYER_ADDR: &str = "relayer";

fn packet_from_data_and_channel(packet: &IbcPacketData, channel_info: &ChannelInfo) -> IbcPacket {
    IbcPacket::new(
        packet.data.clone(),
        IbcEndpoint {
            port_id: packet.src_port_id.clone(),
            channel_id: packet.src_channel_id.clone(),
        },
        IbcEndpoint {
            port_id: channel_info.info.counterparty_endpoint.port_id.to_string(),
            channel_id: packet.dst_channel_id.clone(),
        },
        packet.sequence,
        packet.timeout.clone(),
    )
}

impl IbcSimpleModule {
    fn create_connection(
        &self,
        storage: &mut dyn Storage,
        remote_chain_id: String,
        connection_id: Option<String>,
        counterparty_connection_id: Option<String>,
    ) -> AnyResult<crate::AppResponse> {
        let mut ibc_storage = prefixed(storage, NAMESPACE_IBC);

        // First we get the data (from storage or create it)
        let (connection_id, mut data) = if let Some(connection_id) = connection_id {
            (
                connection_id.clone(),
                ibc_connections().load(&ibc_storage, &connection_id)?,
            )
        } else {
            let connection_count = ibc_connections()
                .range(&ibc_storage, None, None, Order::Ascending)
                .count();
            let connection_id = format!("connection-{}", connection_count);
            (
                connection_id,
                Connection {
                    counterparty_connection_id: None,
                    counterparty_chain_id: remote_chain_id.clone(),
                },
            )
        };

        // We make sure we're not doing weird things
        ensure_eq!(
            remote_chain_id,
            data.counterparty_chain_id,
            anyhow!(
                "Wrong chain id already registered with this connection {}, {}!={}",
                connection_id.clone(),
                data.counterparty_chain_id,
                remote_chain_id
            )
        );

        // We eventually save the counterparty_chain_id
        if let Some(counterparty_connection_id) = counterparty_connection_id {
            data.counterparty_connection_id = Some(counterparty_connection_id);
        }

        // The tx will return the connection id
        ibc_connections().save(&mut ibc_storage, &connection_id, &data)?;

        let event = Event::new("connection_open").add_attribute("connection_id", &connection_id);

        Ok(AppResponse {
            data: None,
            events: vec![event],
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn open_channel<ExecC, QueryC>(
        &self,
        api: &dyn cosmwasm_std::Api,
        storage: &mut dyn Storage,
        router: &dyn crate::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &cosmwasm_std::BlockInfo,
        local_connection_id: String,
        local_port: String,
        version: String,
        order: IbcOrder,

        counterparty_endpoint: IbcEndpoint,
        counterparty_version: Option<String>,
    ) -> AnyResult<crate::AppResponse>
    where
        ExecC: std::fmt::Debug
            + Clone
            + PartialEq
            + schemars::JsonSchema
            + serde::de::DeserializeOwned
            + 'static,
        QueryC: cosmwasm_std::CustomQuery + serde::de::DeserializeOwned + 'static,
    {
        let mut ibc_storage = prefixed(storage, NAMESPACE_IBC);

        // We verify the connection_id exists locally
        if !ibc_connections().has(&ibc_storage, &local_connection_id) {
            bail!(
                "connection {local_connection_id} doesn't exist on chain {}",
                block.chain_id
            )
        };

        // Here we just verify that the port exists locally.
        let port: MockIbcPort = local_port.parse()?;

        // We create a new channel id
        let mut port_info = load_port_info(&ibc_storage, local_port.clone())?;

        let channel_id = format!("channel-{}", port_info.next_channel_id);
        port_info.next_channel_id += 1;

        PORT_INFO.save(&mut ibc_storage, local_port.clone(), &port_info)?;

        let local_endpoint = IbcEndpoint {
            port_id: local_port.clone(),
            channel_id: channel_id.clone(),
        };

        let mut handshake_object = ChannelHandshakeInfo {
            local_endpoint: local_endpoint.clone(),
            remote_endpoint: counterparty_endpoint.clone(),
            state: ChannelHandshakeState::Init,
            version: version.clone(),
            port: port.clone(),
            order: order.clone(),
            connection_id: local_connection_id.clone(),
        };

        let channel = IbcChannel::new(
            local_endpoint.clone(),
            counterparty_endpoint.clone(),
            order.clone(),
            version.clone(),
            local_connection_id.clone(),
        );

        let (open_request, mut ibc_event) = if let Some(counterparty_version) = counterparty_version
        {
            handshake_object.state = ChannelHandshakeState::Try;

            let event = Event::new("channel_open_try");

            (
                IbcChannelOpenMsg::OpenTry {
                    channel,
                    counterparty_version,
                },
                event,
            )
        } else {
            let event = Event::new("channel_open_init");

            (IbcChannelOpenMsg::OpenInit { channel }, event)
        };

        ibc_event = ibc_event
            .add_attribute("port_id", local_endpoint.port_id)
            .add_attribute("channel_id", local_endpoint.channel_id)
            .add_attribute(
                "counterparty_port_id",
                counterparty_endpoint.clone().port_id,
            )
            .add_attribute("counterparty_channel_id", "".to_string())
            .add_attribute("connection_id", local_connection_id);

        // First we send an ibc message on the wasm module in cache
        let res = transactional(storage, |write_cache, _| {
            router.ibc(
                api,
                write_cache,
                block,
                IbcRouterMsg {
                    module: port.into(),
                    msg: super::IbcModuleMsg::ChannelOpen(open_request),
                },
            )
        })?;

        // Then, we store the acknowledgement and collect events
        match res {
            IbcResponse::OpenResponse(r) => {
                // The channel version may be changed here
                let version = r.map(|r| r.version).unwrap_or(version);
                handshake_object.version = version.clone();
                ibc_event = ibc_event.add_attribute("version", version);
                // This is repeated to avoid multiple mutable borrows
                let mut ibc_storage = prefixed(storage, NAMESPACE_IBC);
                // We save the channel handshake status
                CHANNEL_HANDSHAKE_INFO.save(
                    &mut ibc_storage,
                    (local_port, channel_id),
                    &handshake_object,
                )?;
            }
            _ => panic!("Only an open response was expected when receiving a packet"),
        };

        let events = vec![ibc_event];

        Ok(AppResponse { data: None, events })
    }

    fn connect_channel<ExecC, QueryC>(
        &self,
        api: &dyn cosmwasm_std::Api,
        storage: &mut dyn Storage,
        router: &dyn crate::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &cosmwasm_std::BlockInfo,
        port_id: String,
        channel_id: String,

        counterparty_endpoint: IbcEndpoint,
        counterparty_version: Option<String>,
    ) -> AnyResult<crate::AppResponse>
    where
        ExecC: std::fmt::Debug
            + Clone
            + PartialEq
            + schemars::JsonSchema
            + serde::de::DeserializeOwned
            + 'static,
        QueryC: cosmwasm_std::CustomQuery + serde::de::DeserializeOwned + 'static,
    {
        let mut ibc_storage = prefixed(storage, NAMESPACE_IBC);

        // We load the channel handshake info (second step)
        let mut channel_handshake =
            CHANNEL_HANDSHAKE_INFO.load(&ibc_storage, (port_id.clone(), channel_id.clone()))?;

        // We update the remote endpoint
        channel_handshake.remote_endpoint = counterparty_endpoint;

        let channel = IbcChannel::new(
            channel_handshake.local_endpoint.clone(),
            channel_handshake.remote_endpoint.clone(),
            channel_handshake.order.clone(),
            channel_handshake.version.clone(),
            channel_handshake.connection_id.to_string(),
        );

        let (connect_request, mut ibc_event) =
            if channel_handshake.state == ChannelHandshakeState::Try {
                channel_handshake.state = ChannelHandshakeState::Confirm;

                let event = Event::new("channel_open_confirm");

                (IbcChannelConnectMsg::OpenConfirm { channel }, event)
            } else if channel_handshake.state == ChannelHandshakeState::Init {
                // If we were in the init state, now we need to ack the channel creation

                channel_handshake.state = ChannelHandshakeState::Ack;

                let event = Event::new("channel_open_ack");

                (
                    IbcChannelConnectMsg::OpenAck {
                        channel,
                        counterparty_version: counterparty_version.clone().unwrap(), // This should be set in case of an ack
                    },
                    event,
                )
            } else {
                bail!("This is unreachable, configuration error");
            };

        ibc_event = ibc_event
            .add_attribute("port_id", channel_handshake.local_endpoint.port_id.clone())
            .add_attribute(
                "channel_id",
                channel_handshake.local_endpoint.channel_id.clone(),
            )
            .add_attribute(
                "counterparty_port_id",
                channel_handshake.remote_endpoint.port_id.clone(),
            )
            .add_attribute(
                "counterparty_channel_id",
                channel_handshake.remote_endpoint.channel_id.clone(),
            )
            .add_attribute("connection_id", channel_handshake.connection_id.clone());

        // Remove handshake, add channel
        CHANNEL_HANDSHAKE_INFO.remove(&mut ibc_storage, (port_id.clone(), channel_id.clone()));
        CHANNEL_INFO.save(
            &mut ibc_storage,
            (port_id.clone(), channel_id.clone()),
            &ChannelInfo {
                next_packet_id: 1,
                last_packet_relayed: 1,
                info: IbcChannel::new(
                    IbcEndpoint {
                        port_id: port_id.clone(),
                        channel_id: channel_id.clone(),
                    },
                    IbcEndpoint {
                        port_id: channel_handshake.remote_endpoint.port_id.clone(),
                        channel_id: channel_handshake.remote_endpoint.channel_id.clone(),
                    },
                    channel_handshake.order,
                    counterparty_version.unwrap(),
                    channel_handshake.connection_id,
                ),
            },
        )?;

        // First we send an ibc message on the wasm module in cache
        let res = transactional(storage, |write_cache, _| {
            router.ibc(
                api,
                write_cache,
                block,
                IbcRouterMsg {
                    module: channel_handshake.port.into(),
                    msg: super::IbcModuleMsg::ChannelConnect(connect_request),
                },
            )
        })?;

        // Then, we store the acknowledgement and collect events
        let mut events = match res {
            IbcResponse::BasicResponse(r) => r.events,
            _ => panic!("Only an open response was expected when receiving a packet"),
        };

        events.push(ibc_event);

        Ok(AppResponse { data: None, events })
    }

    fn send_packet(
        &self,
        storage: &mut dyn Storage,
        port_id: String,
        channel_id: String,
        data: Binary,
        timeout: IbcTimeout,
    ) -> AnyResult<crate::AppResponse> {
        let mut ibc_storage = prefixed(storage, NAMESPACE_IBC);

        // On this storage, we need to get the id of the transfer packet
        // Get the last packet index

        let mut channel_info =
            CHANNEL_INFO.load(&ibc_storage, (port_id.clone(), channel_id.clone()))?;
        let packet = IbcPacketData {
            ack: None,
            src_channel_id: channel_id.clone(),
            src_port_id: channel_info.info.endpoint.port_id.to_string(),
            dst_channel_id: channel_info.info.counterparty_endpoint.channel_id.clone(),
            dst_port_id: channel_info.info.counterparty_endpoint.port_id.clone(),
            sequence: channel_info.next_packet_id,
            data,
            timeout,
        };
        // Saving this packet for relaying purposes
        SEND_PACKET_MAP.save(
            &mut ibc_storage,
            (
                port_id.clone(),
                channel_id.clone(),
                channel_info.next_packet_id,
            ),
            &packet.clone(),
        )?;

        // Incrementing the packet sequence
        channel_info.next_packet_id += 1;
        CHANNEL_INFO.save(&mut ibc_storage, (port_id, channel_id), &channel_info)?;

        // We add custom packet sending events
        let timeout_height = packet.timeout.block().unwrap_or(IbcTimeoutBlock {
            revision: 0,
            height: 0,
        });
        let timeout_timestamp = packet.timeout.timestamp().map(|t| t.nanos()).unwrap_or(0);

        let send_event = Event::new("send_packet")
            .add_attribute(
                "packet_data",
                String::from_utf8_lossy(packet.data.as_slice()),
            )
            .add_attribute("packet_data_hex", hex::encode(packet.data.0.clone()))
            .add_attribute(
                "packet_timeout_height",
                format!("{}-{}", timeout_height.revision, timeout_height.height),
            )
            .add_attribute("packet_timeout_timestamp", timeout_timestamp.to_string())
            .add_attribute("packet_sequence", packet.sequence.to_string())
            .add_attribute("packet_src_port", packet.src_port_id.clone())
            .add_attribute("packet_src_channel", packet.src_channel_id.clone())
            .add_attribute("packet_dst_port", packet.dst_port_id.clone())
            .add_attribute("packet_dst_channel", packet.dst_channel_id.clone())
            .add_attribute(
                "packet_channel_ordering",
                serde_json::to_value(channel_info.info.order)?.to_string(),
            )
            .add_attribute("packet_connection", channel_info.info.connection_id);

        let events = vec![send_event];
        Ok(AppResponse { data: None, events })
    }

    fn receive_packet<ExecC, QueryC>(
        &self,
        api: &dyn cosmwasm_std::Api,
        storage: &mut dyn Storage,
        router: &dyn crate::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &cosmwasm_std::BlockInfo,
        packet: IbcPacketData,
    ) -> AnyResult<crate::AppResponse>
    where
        ExecC: std::fmt::Debug
            + Clone
            + PartialEq
            + schemars::JsonSchema
            + serde::de::DeserializeOwned
            + 'static,
        QueryC: cosmwasm_std::CustomQuery + serde::de::DeserializeOwned + 'static,
    {
        let mut ibc_storage = prefixed(storage, NAMESPACE_IBC);

        // First we get the channel info to get the port out of it
        let channel_info: ChannelInfo = CHANNEL_INFO.load(
            &ibc_storage,
            (packet.dst_port_id.clone(), packet.dst_channel_id.clone()),
        )?;

        // First we verify it's not already in storage. If its is, we error, not possible to receive the same packet twice
        if RECEIVE_PACKET_MAP
            .load(
                &ibc_storage,
                (
                    packet.dst_port_id.clone(),
                    packet.dst_channel_id.clone(),
                    packet.sequence,
                ),
            )
            .is_ok()
        {
            bail!("You can't receive the same packet twice on the chain")
        }

        // We save it into storage (for tracking purposes and making sure we don't broadcast the message twice)
        RECEIVE_PACKET_MAP.save(
            &mut ibc_storage,
            (
                packet.dst_port_id.clone(),
                packet.dst_channel_id.clone(),
                packet.sequence,
            ),
            &packet,
        )?;

        let packet_msg = packet_from_data_and_channel(&packet, &channel_info);

        #[cfg(not(feature = "cosmwasm_1_1"))]
        let receive_msg = IbcPacketReceiveMsg::new(packet_msg);
        #[cfg(feature = "cosmwasm_1_1")]
        let receive_msg = IbcPacketReceiveMsg::new(packet_msg, Addr::unchecked(RELAYER_ADDR));

        // First we send an ibc message on the wasm module in cache
        let port: MockIbcPort = channel_info.info.endpoint.port_id.parse()?;

        let res = transactional(storage, |write_cache, _| {
            router.ibc(
                api,
                write_cache,
                block,
                IbcRouterMsg {
                    module: port.into(),
                    msg: super::IbcModuleMsg::PacketReceive(receive_msg),
                },
            )
        })?;

        // This is repeated to avoid multiple mutable borrows
        let mut ibc_storage = prefixed(storage, NAMESPACE_IBC);
        let acknowledgement;
        // Then, we store the acknowledgement and collect events
        let mut events = match res {
            IbcResponse::ReceiveResponse(r) => {
                // We save the acknowledgment in the structure
                acknowledgement = r.acknowledgement.clone();
                ACK_PACKET_MAP.save(
                    &mut ibc_storage,
                    (
                        packet.dst_port_id.clone(),
                        packet.dst_channel_id.clone(),
                        packet.sequence,
                    ),
                    &IbcPacketAck {
                        ack: r.acknowledgement,
                    },
                )?;
                r.events
            }
            _ => panic!("Only a receive response was expected when receiving a packet"),
        };

        let timeout_height = packet.timeout.block().unwrap_or(IbcTimeoutBlock {
            revision: 0,
            height: 0,
        });
        let timeout_timestamp = packet.timeout.timestamp().map(|t| t.nanos()).unwrap_or(0);

        let recv_event = Event::new("recv_packet")
            .add_attribute(
                "packet_data",
                String::from_utf8_lossy(packet.data.as_slice()),
            )
            .add_attribute("packet_data_hex", hex::encode(packet.data.0.clone()))
            .add_attribute(
                "packet_timeout_height",
                format!("{}-{}", timeout_height.revision, timeout_height.height),
            )
            .add_attribute("packet_timeout_timestamp", timeout_timestamp.to_string())
            .add_attribute("packet_sequence", packet.sequence.to_string())
            .add_attribute("packet_src_port", packet.src_port_id.clone())
            .add_attribute("packet_src_channel", packet.src_channel_id.clone())
            .add_attribute("packet_dst_port", packet.dst_port_id.clone())
            .add_attribute("packet_dst_channel", packet.dst_channel_id.clone())
            .add_attribute(
                "packet_channel_ordering",
                serde_json::to_value(channel_info.info.order)?.to_string(),
            )
            .add_attribute("packet_connection", channel_info.info.connection_id);

        let ack_event = Event::new("write_acknowledgement")
            .add_attribute(
                "packet_data",
                serde_json::to_value(&packet.data)?.to_string(),
            )
            .add_attribute("packet_data_hex", hex::encode(packet.data.0))
            .add_attribute(
                "packet_timeout_height",
                format!("{}-{}", timeout_height.revision, timeout_height.height),
            )
            .add_attribute("packet_timeout_timestamp", timeout_timestamp.to_string())
            .add_attribute("packet_sequence", packet.sequence.to_string())
            .add_attribute("packet_src_port", packet.src_port_id)
            .add_attribute("packet_src_channel", packet.src_channel_id)
            .add_attribute("packet_dst_port", packet.dst_port_id)
            .add_attribute("packet_dst_channel", packet.dst_channel_id)
            .add_attribute(
                "packet_ack",
                String::from_utf8_lossy(acknowledgement.as_slice()),
            )
            .add_attribute("packet_ack_hex", hex::encode(acknowledgement.0));

        events.push(recv_event);
        events.push(ack_event);

        Ok(AppResponse { data: None, events })
    }

    fn acknowledge_packet<ExecC, QueryC>(
        &self,
        api: &dyn cosmwasm_std::Api,
        storage: &mut dyn Storage,
        router: &dyn crate::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &cosmwasm_std::BlockInfo,
        packet: IbcPacketData,
        ack: Binary,
    ) -> AnyResult<crate::AppResponse>
    where
        ExecC: std::fmt::Debug
            + Clone
            + PartialEq
            + schemars::JsonSchema
            + serde::de::DeserializeOwned
            + 'static,
        QueryC: cosmwasm_std::CustomQuery + serde::de::DeserializeOwned + 'static,
    {
        let mut ibc_storage = prefixed(storage, NAMESPACE_IBC);

        // First we get the channel info to get the port out of it
        let channel_info = CHANNEL_INFO.load(
            &ibc_storage,
            (packet.src_port_id.clone(), packet.src_channel_id.clone()),
        )?;

        // First we verify the packet exists and the acknowledgement is not received yet
        let mut packet_data: IbcPacketData = SEND_PACKET_MAP.load(
            &ibc_storage,
            (
                packet.src_port_id.clone(),
                packet.src_channel_id.clone(),
                packet.sequence,
            ),
        )?;
        if packet_data.ack.is_some() {
            bail!("You can't ack the same packet twice on the chain")
        }

        if TIMEOUT_PACKET_MAP.has(
            &ibc_storage,
            (
                packet.src_port_id.clone(),
                packet.src_channel_id.clone(),
                packet.sequence,
            ),
        ) {
            bail!("Packet has timed_out, can't acknowledge");
        }

        // We save the ack into storage
        packet_data.ack = Some(ack.clone());
        SEND_PACKET_MAP.save(
            &mut ibc_storage,
            (
                packet.src_port_id.clone(),
                packet.src_channel_id.clone(),
                packet.sequence,
            ),
            &packet_data,
        )?;

        let acknowledgement = IbcAcknowledgement::new(ack);
        let original_packet = packet_from_data_and_channel(&packet_data, &channel_info);

        #[cfg(not(feature = "cosmwasm_1_1"))]
        let ack_message = IbcPacketAckMsg::new(acknowledgement, original_packet);
        #[cfg(feature = "cosmwasm_1_1")]
        let ack_message = IbcPacketAckMsg::new(
            acknowledgement,
            original_packet,
            Addr::unchecked(RELAYER_ADDR),
        );

        let port: MockIbcPort = channel_info.info.endpoint.port_id.parse()?;
        let res = transactional(storage, |write_cache, _| {
            router.ibc(
                api,
                write_cache,
                block,
                IbcRouterMsg {
                    module: port.into(),
                    msg: super::IbcModuleMsg::PacketAcknowledgement(ack_message),
                },
            )
        })?;

        let mut events = match res {
            // Only type allowed as an ack response
            IbcResponse::BasicResponse(r) => r.events,
            _ => panic!("Only a basic response was expected when ack a packet"),
        };

        // We add custom packet ack events
        let timeout_height = packet.timeout.block().unwrap_or(IbcTimeoutBlock {
            revision: 0,
            height: 0,
        });
        let timeout_timestamp = packet.timeout.timestamp().map(|t| t.nanos()).unwrap_or(0);

        let ack_event = Event::new("recv_packet")
            .add_attribute(
                "packet_timeout_height",
                format!("{}-{}", timeout_height.revision, timeout_height.height),
            )
            .add_attribute("packet_timeout_timestamp", timeout_timestamp.to_string())
            .add_attribute("packet_sequence", packet.sequence.to_string())
            .add_attribute("packet_src_port", packet.src_port_id.clone())
            .add_attribute("packet_src_channel", packet.src_channel_id.clone())
            .add_attribute("packet_dst_port", packet.dst_port_id.clone())
            .add_attribute("packet_dst_channel", packet.dst_channel_id.clone())
            .add_attribute(
                "packet_channel_ordering",
                serde_json::to_value(channel_info.info.order)?.to_string(),
            )
            .add_attribute("packet_connection", channel_info.info.connection_id);

        events.push(ack_event);

        Ok(AppResponse { data: None, events })
    }

    fn timeout_packet<ExecC, QueryC>(
        &self,
        api: &dyn cosmwasm_std::Api,
        storage: &mut dyn Storage,
        router: &dyn crate::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &cosmwasm_std::BlockInfo,
        packet: IbcPacketData,
    ) -> AnyResult<crate::AppResponse>
    where
        ExecC: std::fmt::Debug
            + Clone
            + PartialEq
            + schemars::JsonSchema
            + serde::de::DeserializeOwned
            + 'static,
        QueryC: cosmwasm_std::CustomQuery + serde::de::DeserializeOwned + 'static,
    {
        let mut ibc_storage = prefixed(storage, NAMESPACE_IBC);

        // First we get the channel info to get the port out of it
        let channel_info = CHANNEL_INFO.load(
            &ibc_storage,
            (packet.src_port_id.clone(), packet.src_channel_id.clone()),
        )?;

        // We verify the timeout is indeed passed on the packet
        let packet_data: IbcPacketData = SEND_PACKET_MAP.load(
            &ibc_storage,
            (
                packet.src_port_id.clone(),
                packet.src_channel_id.clone(),
                packet.sequence,
            ),
        )?;

        // If the packet was already aknowledge, no timeout possible
        if packet_data.ack.is_some() {
            bail!("You can't timeout an acked packet")
        }

        if TIMEOUT_PACKET_MAP
            .may_load(
                &ibc_storage,
                (
                    packet.src_port_id.clone(),
                    packet.src_channel_id.clone(),
                    packet.sequence,
                ),
            )?
            .is_some()
        {
            bail!("You can't timeout an packet twice")
        }

        // If there is a block timeout
        let mut has_timedout = false;
        if let Some(block_timeout) = packet_data.timeout.block() {
            if block.height >= block_timeout.height {
                has_timedout = true;
            }
        }
        if let Some(timeout) = packet_data.timeout.timestamp() {
            if block.time >= timeout {
                has_timedout = true;
            }
        }

        if !has_timedout {
            bail!("Packet hasn't timedout");
        }

        TIMEOUT_PACKET_MAP.save(
            &mut ibc_storage,
            (
                packet.src_port_id.clone(),
                packet.src_channel_id.clone(),
                packet.sequence,
            ),
            &true,
        )?;

        let original_packet = packet_from_data_and_channel(&packet_data, &channel_info);

        #[cfg(not(feature = "cosmwasm_1_1"))]
        let timeout_message = IbcPacketTimeoutMsg::new(original_packet);
        #[cfg(feature = "cosmwasm_1_1")]
        let timeout_message =
            IbcPacketTimeoutMsg::new(original_packet, Addr::unchecked(RELAYER_ADDR));

        // First we send an ibc message on the module in cache
        let port: MockIbcPort = channel_info.info.endpoint.port_id.parse()?;
        let res = transactional(storage, |write_cache, _| {
            router.ibc(
                api,
                write_cache,
                block,
                IbcRouterMsg {
                    module: port.into(),
                    msg: super::IbcModuleMsg::PacketTimeout(timeout_message),
                },
            )
        })?;

        // Then we collect events
        let mut events = match res {
            // Only type allowed as an timeout response
            IbcResponse::BasicResponse(r) => r.events,
            _ => panic!("Only a basic response was expected when timeout a packet"),
        };

        // We add custom packet timeout events
        let timeout_height = packet.timeout.block().unwrap_or(IbcTimeoutBlock {
            revision: 0,
            height: 0,
        });
        let timeout_timestamp = packet.timeout.timestamp().map(|t| t.nanos()).unwrap_or(0);

        let timeout_event = Event::new("timeout_packet")
            .add_attribute(
                "packet_timeout_height",
                format!("{}-{}", timeout_height.revision, timeout_height.height),
            )
            .add_attribute("packet_timeout_timestamp", timeout_timestamp.to_string())
            .add_attribute("packet_sequence", packet.sequence.to_string())
            .add_attribute("packet_src_port", packet.src_port_id.clone())
            .add_attribute("packet_src_channel", packet.src_channel_id.clone())
            .add_attribute("packet_dst_port", packet.dst_port_id.clone())
            .add_attribute("packet_dst_channel", packet.dst_channel_id.clone())
            .add_attribute(
                "packet_channel_ordering",
                serde_json::to_value(channel_info.info.order)?.to_string(),
            );

        events.push(timeout_event);

        Ok(AppResponse { data: None, events })
    }

    // Applications
    fn transfer<ExecC, QueryC>(
        &self,
        api: &dyn cosmwasm_std::Api,
        storage: &mut dyn cosmwasm_std::Storage,
        router: &dyn crate::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &cosmwasm_std::BlockInfo,
        sender: Addr,
        channel_id: String,
        to_address: String,
        amount: Coin,
        timeout: IbcTimeout,
    ) -> AnyResult<crate::AppResponse>
    where
        ExecC: std::fmt::Debug
            + Clone
            + PartialEq
            + schemars::JsonSchema
            + serde::de::DeserializeOwned
            + 'static,
        QueryC: cosmwasm_std::CustomQuery + serde::de::DeserializeOwned + 'static,
    {
        // Transfer is  :
        // 1. Lock user funds into the port balance. We send from the sender to the lock address
        let msg: cosmwasm_std::CosmosMsg<ExecC> = BankMsg::Send {
            to_address: IBC_LOCK_MODULE_ADDRESS.to_string(),
            amount: vec![amount.clone()],
        }
        .into();
        router.execute(api, storage, block, sender.clone(), msg)?;

        // We unwrap the denom if the funds were received on this specific channel
        let denom = optional_unwrap_ibc_denom(amount.denom, channel_id.clone());

        // 2. Send an ICS20 Packet to the remote chain
        let packet_formed = Ics20Packet {
            amount: amount.amount,
            denom,
            receiver: to_address,
            sender: sender.to_string(),
            memo: None,
        };

        self.send_packet(
            storage,
            "transfer".to_string(),
            channel_id,
            to_json_binary(&packet_formed)?,
            timeout,
        )
    }

    pub fn close_channel(
        &self,
        _storage: &mut dyn Storage,
        _channel_id: String,
    ) -> AnyResult<crate::AppResponse> {
        bail!("Close channel not implemented in cw-multi-test");
    }
}

impl Module for IbcSimpleModule {
    type ExecT = IbcMsg;
    type QueryT = MockIbcQuery;
    type SudoT = IbcPacketRelayingMsg;

    fn execute<ExecC, QueryC>(
        &self,
        api: &dyn cosmwasm_std::Api,
        storage: &mut dyn cosmwasm_std::Storage,
        router: &dyn crate::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &cosmwasm_std::BlockInfo,
        sender: cosmwasm_std::Addr,
        msg: Self::ExecT,
    ) -> anyhow::Result<crate::AppResponse>
    where
        ExecC: std::fmt::Debug
            + Clone
            + PartialEq
            + schemars::JsonSchema
            + serde::de::DeserializeOwned
            + 'static,
        QueryC: cosmwasm_std::CustomQuery + serde::de::DeserializeOwned + 'static,
    {
        match msg {
            IbcMsg::Transfer {
                channel_id,
                to_address,
                amount,
                timeout,
            } => self.transfer(
                api, storage, router, block, sender, channel_id, to_address, amount, timeout,
            ),
            IbcMsg::SendPacket {
                channel_id,
                data,
                timeout,
            } => {
                // This should come from a contract. So the port_id is always the same format
                // If you want to send a packet form a module use the sudo Send Packet msg
                let port_id = format!("wasm.{}", sender);
                self.send_packet(storage, port_id, channel_id, data, timeout)
            }
            IbcMsg::CloseChannel { channel_id } => self.close_channel(storage, channel_id),
            _ => bail!("Not implemented on the ibc module"),
        }
    }

    fn sudo<ExecC, QueryC>(
        &self,
        api: &dyn cosmwasm_std::Api,
        storage: &mut dyn cosmwasm_std::Storage,
        router: &dyn crate::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &cosmwasm_std::BlockInfo,
        msg: Self::SudoT,
    ) -> anyhow::Result<crate::AppResponse>
    where
        ExecC: std::fmt::Debug
            + Clone
            + PartialEq
            + schemars::JsonSchema
            + serde::de::DeserializeOwned
            + 'static,
        QueryC: cosmwasm_std::CustomQuery + serde::de::DeserializeOwned + 'static,
    {
        let response = match msg {
            IbcPacketRelayingMsg::CreateConnection {
                connection_id,
                remote_chain_id,
                counterparty_connection_id,
            } => self.create_connection(
                storage,
                remote_chain_id,
                connection_id,
                counterparty_connection_id,
            ),

            IbcPacketRelayingMsg::OpenChannel {
                local_connection_id,
                local_port,
                version,
                order,
                counterparty_version,
                counterparty_endpoint,
            } => self.open_channel(
                api,
                storage,
                router,
                block,
                local_connection_id,
                local_port,
                version,
                order,
                counterparty_endpoint,
                counterparty_version,
            ),
            IbcPacketRelayingMsg::ConnectChannel {
                counterparty_version,
                counterparty_endpoint,
                port_id,
                channel_id,
            } => self.connect_channel(
                api,
                storage,
                router,
                block,
                port_id,
                channel_id,
                counterparty_endpoint,
                counterparty_version,
            ),
            IbcPacketRelayingMsg::CloseChannel {} => {
                panic!("Can't close a channel in cw-multi-test")
            }

            IbcPacketRelayingMsg::Send {
                port_id,
                channel_id,
                data,
                timeout,
            } => self.send_packet(storage, port_id, channel_id, data, timeout),
            IbcPacketRelayingMsg::Receive { packet } => {
                self.receive_packet(api, storage, router, block, packet)
            }
            IbcPacketRelayingMsg::Acknowledge { packet, ack } => {
                self.acknowledge_packet(api, storage, router, block, packet, ack)
            }
            IbcPacketRelayingMsg::Timeout { packet } => {
                self.timeout_packet(api, storage, router, block, packet)
            }
        }?;

        Ok(response)
    }

    fn query(
        &self,
        _api: &dyn cosmwasm_std::Api,
        storage: &dyn cosmwasm_std::Storage,
        _querier: &dyn cosmwasm_std::Querier,
        _block: &cosmwasm_std::BlockInfo,
        request: Self::QueryT,
    ) -> anyhow::Result<cosmwasm_std::Binary> {
        let ibc_storage = prefixed_read(storage, NAMESPACE_IBC);
        match request {
            MockIbcQuery::CosmWasm(m) => {
                match m {
                    IbcQuery::Channel {
                        channel_id,
                        port_id,
                    } => {
                        // Port id has to be specificed unfortunately here
                        let port_id = port_id.unwrap();
                        // We load the channel of the port
                        let channel_info =
                            CHANNEL_INFO.may_load(&ibc_storage, (port_id, channel_id.clone()))?;

                        Ok(to_json_binary(&ChannelResponse {
                            channel: channel_info.map(|c| c.info),
                        })?)
                    }
                    IbcQuery::ListChannels { port_id } => {
                        // Port_id has to be specified here, unfortunately we can't access the contract address
                        let port_id = port_id.unwrap();

                        let channels = CHANNEL_INFO
                            .prefix(port_id)
                            .range(&ibc_storage, None, None, Order::Ascending)
                            .collect::<Result<Vec<_>, _>>()?;

                        Ok(to_json_binary(&ListChannelsResponse {
                            channels: channels.iter().map(|c| c.1.info.clone()).collect(),
                        })?)
                    }
                    _ => bail!("Query not available"),
                }
            }
            MockIbcQuery::SendPacket {
                channel_id,
                port_id,
                sequence,
            } => {
                let packet_data =
                    SEND_PACKET_MAP.load(&ibc_storage, (port_id, channel_id, sequence))?;

                Ok(to_json_binary(&packet_data)?)
            }
            MockIbcQuery::ConnectedChain { connection_id } => {
                let chain_id = ibc_connections().load(&ibc_storage, &connection_id)?;

                Ok(to_json_binary(&chain_id)?)
            }
            MockIbcQuery::ChainConnections { chain_id } => {
                let connections = ibc_connections()
                    .idx
                    .chain_id
                    .prefix(chain_id)
                    .range(&ibc_storage, None, None, Order::Descending)
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(to_json_binary(&connections)?)
            }
        }
    }

    //Ibc endpoints are not available on the IBC module. This module is only a fix for receiving IBC messages. The IBC module doesn't and will never have ports opened to other blockchains
}

impl Ibc for IbcSimpleModule {}

#[cfg(test)]
mod test {}
