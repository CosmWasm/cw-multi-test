//! Ibc Module adds IBC support to cw-multi-test
#![allow(missing_docs)]
use cosmwasm_std::{
    IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcMsg, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg,
};

use crate::{AcceptingModule, FailingModule, Module};

pub mod events;
pub mod relayer;
mod simple_ibc;
mod state;
pub mod types;
pub use self::types::IbcPacketRelayingMsg;
use self::types::MockIbcQuery;
pub use simple_ibc::IbcSimpleModule;

/// This is added for modules to implement actions upon ibc actions.
/// This kind of execution flow is copied from the WASM way of doing things and is not 100% completetely compatible with the IBC standard
/// Those messages should only be called by the Ibc module.
/// For additional Modules, the packet endpoints should be implemented
/// The Channel endpoints are usually not implemented besides storing the channel ids
#[cosmwasm_schema::cw_serde]
pub enum IbcModuleMsg {
    /// Open an IBC Channel (2 first steps)
    ChannelOpen(IbcChannelOpenMsg),
    /// Connect an IBC Channel (2 last steps)
    ChannelConnect(IbcChannelConnectMsg),
    /// Close an IBC Channel
    ChannelClose(IbcChannelCloseMsg),

    /// Receive an IBC Packet
    PacketReceive(IbcPacketReceiveMsg),
    /// Receive an IBC Acknowledgement for a packet
    PacketAcknowledgement(IbcPacketAckMsg),
    /// Receive an IBC Timeout for a packet
    PacketTimeout(IbcPacketTimeoutMsg),
}
///Manages Inter-Blockchain Communication (IBC) functionalities.
///This trait is critical for testing contracts that involve cross-chain interactions,
///reflecting the interconnected nature of the Cosmos ecosystem.
pub trait Ibc: Module<ExecT = IbcMsg, QueryT = MockIbcQuery, SudoT = IbcPacketRelayingMsg> {}
/// Ideal for testing contracts that involve IBC, this module is designed to successfully
/// handle cross-chain messages. It's key for ensuring that your contract can smoothly interact
/// with other blockchains in the Cosmos network.
pub type IbcAcceptingModule = AcceptingModule<IbcMsg, MockIbcQuery, IbcPacketRelayingMsg>;

impl Ibc for IbcAcceptingModule {}
/// Use this to test how your contract deals with problematic IBC scenarios.
/// It's a module that deliberately fails in handling IBC messages, allowing you
/// to check how your contract behaves in less-than-ideal cross-chain communication situations.
pub type IbcFailingModule = FailingModule<IbcMsg, MockIbcQuery, IbcPacketRelayingMsg>;

impl Ibc for IbcFailingModule {}
