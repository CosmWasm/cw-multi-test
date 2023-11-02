use cosmwasm_std::Storage;
use cw_storage_plus::{Index, IndexList, IndexedMap, Map, MultiIndex};

use super::types::*;

use anyhow::Result as AnyResult;

pub const NAMESPACE_IBC: &[u8] = b"ibc-namespace";

/// This maps a connection id to a remote chain id
pub struct ConnectionIndexes<'a> {
    // chain_id, Connection info, connection_id
    pub chain_id: MultiIndex<'a, String, Connection, String>,
}

impl<'a> IndexList<Connection> for ConnectionIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Connection>> + '_> {
        let v: Vec<&dyn Index<Connection>> = vec![&self.chain_id];
        Box::new(v.into_iter())
    }
}

pub fn ibc_connections<'a>() -> IndexedMap<'a, &'a str, Connection, ConnectionIndexes<'a>> {
    let indexes = ConnectionIndexes {
        chain_id: MultiIndex::new(
            |_, d: &Connection| d.counterparty_chain_id.clone(),
            "connections",
            "connections_chain_id",
        ),
    };
    IndexedMap::new("tokens", indexes)
}

pub const PORT_INFO: Map<String, PortInfo> = Map::new("port_info");

pub const CHANNEL_HANDSHAKE_INFO: Map<(String, String), ChannelHandshakeInfo> =
    Map::new("channel_handshake_info");
pub const CHANNEL_INFO: Map<(String, String), ChannelInfo> = Map::new("channel_info");

// channel id, packet_id ==> Packet data
pub const SEND_PACKET_MAP: Map<(String, String, u64), IbcPacketData> = Map::new("send_packet");

// channel id, packet_id ==> Packet data
pub const RECEIVE_PACKET_MAP: Map<(String, String, u64), IbcPacketData> =
    Map::new("receive_packet");

// channel id, packet_id ==> Packet data
pub const ACK_PACKET_MAP: Map<(String, String, u64), IbcPacketAck> = Map::new("ack_packet");

// channel id, packet_id ==> Packet data
pub const TIMEOUT_PACKET_MAP: Map<(String, String, u64), bool> = Map::new("timeout_packet");

pub fn load_port_info(storage: &dyn Storage, port_id: String) -> AnyResult<PortInfo> {
    Ok(PORT_INFO.may_load(storage, port_id)?.unwrap_or_default())
}
