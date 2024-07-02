use anyhow::Result as AnyResult;
use cosmwasm_std::{from_json, Api, Binary, CustomMsg, CustomQuery, Storage};
use serde::de::DeserializeOwned;

use crate::{
    ibc::{
        events::{
            CHANNEL_CLOSE_INIT_EVENT, SEND_PACKET_EVENT, TIMEOUT_RECEIVE_PACKET_EVENT,
            WRITE_ACK_EVENT,
        },
        types::{IbcPacketData, MockIbcQuery},
        IbcPacketRelayingMsg,
    },
    App, AppResponse, Bank, Distribution, Gov, Ibc, Module, Staking, SudoMsg, Wasm,
};

use super::{get_all_event_attr_value, get_event_attr_value, has_event};

#[derive(Debug, Clone)]
pub struct RelayPacketResult {
    pub receive_tx: AppResponse,
    pub result: RelayingResult,
}

#[derive(Debug, Clone)]
pub enum RelayingResult {
    Timeout {
        timeout_tx: AppResponse,
        close_channel_confirm: Option<AppResponse>,
    },
    Acknowledgement {
        tx: AppResponse,
        ack: Binary,
    },
}

pub fn relay_packets_in_tx<
    BankT1,
    ApiT1,
    StorageT1,
    CustomT1,
    WasmT1,
    StakingT1,
    DistrT1,
    IbcT1,
    GovT1,
    BankT2,
    ApiT2,
    StorageT2,
    CustomT2,
    WasmT2,
    StakingT2,
    DistrT2,
    IbcT2,
    GovT2,
>(
    app1: &mut App<BankT1, ApiT1, StorageT1, CustomT1, WasmT1, StakingT1, DistrT1, IbcT1, GovT1>,
    app2: &mut App<BankT2, ApiT2, StorageT2, CustomT2, WasmT2, StakingT2, DistrT2, IbcT2, GovT2>,
    app1_tx_response: AppResponse,
) -> AnyResult<Vec<RelayPacketResult>>
where
    CustomT1::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT1::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT1: Wasm<CustomT1::ExecT, CustomT1::QueryT>,
    BankT1: Bank,
    ApiT1: Api,
    StorageT1: Storage,
    CustomT1: Module,
    StakingT1: Staking,
    DistrT1: Distribution,
    IbcT1: Ibc,
    GovT1: Gov,

    CustomT2::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT2::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT2: Wasm<CustomT2::ExecT, CustomT2::QueryT>,
    BankT2: Bank,
    ApiT2: Api,
    StorageT2: Storage,
    CustomT2: Module,
    StakingT2: Staking,
    DistrT2: Distribution,
    IbcT2: Ibc,
    GovT2: Gov,
{
    // Find all packets and their data
    let packets = get_all_event_attr_value(&app1_tx_response, SEND_PACKET_EVENT, "packet_sequence");
    let channels =
        get_all_event_attr_value(&app1_tx_response, SEND_PACKET_EVENT, "packet_src_channel");
    let ports = get_all_event_attr_value(&app1_tx_response, SEND_PACKET_EVENT, "packet_src_port");

    // For all packets, query the packetdata and relay them

    let mut packet_forwarding = vec![];

    for i in 0..packets.len() {
        let relay_response = relay_packet(
            app1,
            app2,
            ports[i].clone(),
            channels[i].clone(),
            packets[i].parse()?,
        )?;

        packet_forwarding.push(relay_response);
    }

    Ok(packet_forwarding)
}

/// Relays (rcv + ack) any pending packet between 2 chains
pub fn relay_packet<
    BankT1,
    ApiT1,
    StorageT1,
    CustomT1,
    WasmT1,
    StakingT1,
    DistrT1,
    IbcT1,
    GovT1,
    BankT2,
    ApiT2,
    StorageT2,
    CustomT2,
    WasmT2,
    StakingT2,
    DistrT2,
    IbcT2,
    GovT2,
>(
    app1: &mut App<BankT1, ApiT1, StorageT1, CustomT1, WasmT1, StakingT1, DistrT1, IbcT1, GovT1>,
    app2: &mut App<BankT2, ApiT2, StorageT2, CustomT2, WasmT2, StakingT2, DistrT2, IbcT2, GovT2>,
    src_port_id: String,
    src_channel_id: String,
    sequence: u64,
) -> AnyResult<RelayPacketResult>
where
    CustomT1::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT1::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT1: Wasm<CustomT1::ExecT, CustomT1::QueryT>,
    BankT1: Bank,
    ApiT1: Api,
    StorageT1: Storage,
    CustomT1: Module,
    StakingT1: Staking,
    DistrT1: Distribution,
    IbcT1: Ibc,
    GovT1: Gov,

    CustomT2::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT2::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT2: Wasm<CustomT2::ExecT, CustomT2::QueryT>,
    BankT2: Bank,
    ApiT2: Api,
    StorageT2: Storage,
    CustomT2: Module,
    StakingT2: Staking,
    DistrT2: Distribution,
    IbcT2: Ibc,
    GovT2: Gov,
{
    let packet: IbcPacketData = from_json(app1.ibc_query(MockIbcQuery::SendPacket {
        channel_id: src_channel_id.clone(),
        port_id: src_port_id.clone(),
        sequence,
    })?)?;

    // First we start by sending the packet on chain 2
    let receive_response = app2.sudo(SudoMsg::Ibc(IbcPacketRelayingMsg::Receive {
        packet: packet.clone(),
    }))?;

    // We start by verifying that we have an acknowledgment and not a timeout
    if has_event(&receive_response, TIMEOUT_RECEIVE_PACKET_EVENT) {
        // If there was a timeout, we timeout the packet on the sending chain
        // TODO: We don't handle the chain closure in here for now in case of ordered channels
        let timeout_response = app1.sudo(SudoMsg::Ibc(IbcPacketRelayingMsg::Timeout { packet }))?;

        // We close the channel on the sending chain if it's request by the receiving chain
        let close_confirm_response = if has_event(&receive_response, CHANNEL_CLOSE_INIT_EVENT) {
            Some(app1.sudo(SudoMsg::Ibc(IbcPacketRelayingMsg::CloseChannel {
                port_id: src_port_id,
                channel_id: src_channel_id,
                init: false,
            }))?)
        } else {
            None
        };

        return Ok(RelayPacketResult {
            receive_tx: receive_response,
            result: RelayingResult::Timeout {
                timeout_tx: timeout_response,
                close_channel_confirm: close_confirm_response,
            },
        });
    }

    // Then we query the packet ack to deliver the response on chain 1
    let hex_ack = get_event_attr_value(&receive_response, WRITE_ACK_EVENT, "packet_ack_hex")?;

    let ack = Binary::from(hex::decode(hex_ack)?);

    let ack_response = app1.sudo(SudoMsg::Ibc(IbcPacketRelayingMsg::Acknowledge {
        packet,
        ack: ack.clone(),
    }))?;

    Ok(RelayPacketResult {
        receive_tx: receive_response,
        result: RelayingResult::Acknowledgement {
            tx: ack_response,
            ack,
        },
    })
}
