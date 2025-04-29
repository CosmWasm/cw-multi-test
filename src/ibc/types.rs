use crate::app::IbcModule;
use anyhow::bail;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    Addr, Binary, Event, IbcChannel, IbcChannelOpenResponse, IbcEndpoint, IbcOrder, IbcQuery,
    IbcTimeout,
};
use std::{fmt::Display, str::FromStr};

#[cw_serde]
/// IBC connection
pub struct Connection {
    /// Connection id on the counterparty chain
    pub counterparty_connection_id: Option<String>,
    /// Chain id of the counterparty chain
    pub counterparty_chain_id: String,
}

#[cw_serde]
#[derive(Default)]
/// IBC Port Info
pub struct PortInfo {
    /// Channel id of the next opened channel
    pub next_channel_id: u64,
}

#[cw_serde]
pub struct ChannelHandshakeInfo {
    pub connection_id: String,
    pub port: MockIbcPort,
    pub local_endpoint: IbcEndpoint,
    pub remote_endpoint: IbcEndpoint,
    pub state: ChannelHandshakeState,
    pub order: IbcOrder,
    pub version: String,
}

#[cw_serde]
pub enum ChannelHandshakeState {
    Init,
    Try,
    Ack,
    Confirm,
}

#[cw_serde]
pub struct ChannelInfo {
    pub next_packet_id: u64,
    pub last_packet_relayed: u64,

    pub info: IbcChannel,

    pub open: bool,
}

#[cw_serde]
pub enum MockIbcPort {
    Wasm(String), // A wasm port is simply a wasm contract address
    Bank,         // The bank port simply talks to the bank module
    Staking,      // The staking port simply talks to the staking module
}

impl From<MockIbcPort> for IbcModule {
    fn from(port: MockIbcPort) -> IbcModule {
        match port {
            MockIbcPort::Bank => IbcModule::Bank,
            MockIbcPort::Staking => IbcModule::Staking,
            MockIbcPort::Wasm(contract) => IbcModule::Wasm(Addr::unchecked(contract)),
        }
    }
}

pub const BANK_MODULE_PORT: &str = "transfer";

impl Display for MockIbcPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MockIbcPort::Wasm(c) => write!(f, "wasm.{}", c),
            MockIbcPort::Bank => write!(f, "{BANK_MODULE_PORT}"),
            MockIbcPort::Staking => panic!("No ibc port for the staking module"),
        }
    }
}

impl FromStr for MockIbcPort {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // For the bank module
        if s.eq(BANK_MODULE_PORT) {
            return Ok(MockIbcPort::Bank);
        }

        // For the wasm module
        let wasm = s.split('.').collect::<Vec<_>>();
        if wasm.len() == 2 && wasm[0] == "wasm" {
            return Ok(MockIbcPort::Wasm(wasm[1].to_string()));
        }
        // Error
        bail!(
            "The ibc port {} can't be linked to an mock ibc implementation",
            s
        )
    }
}

#[cw_serde]
pub struct IbcPacketData {
    pub ack: Option<Binary>,
    /// This also tells us whether this packet was already sent on the other chain or not
    pub src_port_id: String,
    pub src_channel_id: String,
    pub dst_port_id: String,
    pub dst_channel_id: String,
    pub sequence: u64,
    pub data: Binary,
    pub timeout: IbcTimeout,
}

#[cw_serde]
pub struct IbcPacketReceived {
    pub data: IbcPacketData,
    /// Indicates whether the packet was received with a timeout
    pub timeout: bool,
}

#[cw_serde]
pub struct IbcPacketAck {
    pub ack: Option<Binary>,
}

/// This is a custom msg that is used for executing actions on the IBC module
/// We trust all packets that are relayed. Remember, this is a test environment.
#[cw_serde]
pub enum IbcPacketRelayingMsg {
    CreateConnection {
        remote_chain_id: String,
        // And in the case we need to register the counterparty id as well
        connection_id: Option<String>,
        counterparty_connection_id: Option<String>,
    },

    OpenChannel {
        local_connection_id: String,
        local_port: String,
        version: String,
        order: IbcOrder,

        counterparty_version: Option<String>,
        counterparty_endpoint: IbcEndpoint,
    },
    ConnectChannel {
        port_id: String,
        channel_id: String,

        counterparty_version: Option<String>,
        counterparty_endpoint: IbcEndpoint,
    },
    CloseChannel {
        port_id: String,
        channel_id: String,
        init: bool,
    },
    Send {
        port_id: String,
        channel_id: String,
        data: Binary,
        timeout: IbcTimeout,
    },
    Receive {
        packet: IbcPacketData,
    },
    Acknowledge {
        packet: IbcPacketData,
        ack: Binary,
    },
    Timeout {
        packet: IbcPacketData,
    },
}

/// This type allows to wrap the IBC response to return from the Router.
#[cw_serde]
pub enum IbcResponse {
    Open(IbcChannelOpenResponse),
    Basic(AppIbcBasicResponse),
    Receive(AppIbcReceiveResponse),
}

#[cw_serde]
#[derive(Default)]
pub struct AppIbcBasicResponse {
    pub events: Vec<Event>,
}

#[cw_serde]
#[derive(Default)]
pub struct AppIbcReceiveResponse {
    pub events: Vec<Event>,
    pub acknowledgement: Option<Binary>,
}

impl From<IbcChannelOpenResponse> for IbcResponse {
    fn from(c: IbcChannelOpenResponse) -> IbcResponse {
        IbcResponse::Open(c)
    }
}

impl From<AppIbcBasicResponse> for IbcResponse {
    fn from(c: AppIbcBasicResponse) -> IbcResponse {
        IbcResponse::Basic(c)
    }
}

impl From<AppIbcReceiveResponse> for IbcResponse {
    fn from(c: AppIbcReceiveResponse) -> IbcResponse {
        IbcResponse::Receive(c)
    }
}

/// This extends the cosmwasm-std IBC query type with internal tools needed.
#[cw_serde]
pub enum MockIbcQuery {
    CosmWasm(IbcQuery),
    /// Only used inside cw-multi-test
    /// Queries a packet that was sent on the chain
    /// Returns `IbcPacketData`
    SendPacket {
        channel_id: String,
        port_id: String,
        sequence: u64,
    },
    /// This is used to get the chain_id of the connected chain
    ConnectedChain {
        connection_id: String,
    },
    /// Gets all the connections with a chain
    ChainConnections {
        chain_id: String,
    },
    /// Gets information on a channel
    ChannelInfo {
        port_id: String,
        channel_id: String,
    },
}

impl From<IbcQuery> for MockIbcQuery {
    fn from(value: IbcQuery) -> Self {
        MockIbcQuery::CosmWasm(value)
    }
}
