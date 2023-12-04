use cosmwasm_std::{
    IbcChannelCloseMsg, IbcChannelConnectMsg, IbcChannelOpenMsg, IbcMsg, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg,
};

use crate::{AcceptingModule, FailingModule, Module};

pub mod addresses;
pub mod api;
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
    ChannelOpen(IbcChannelOpenMsg),
    ChannelConnect(IbcChannelConnectMsg),
    ChannelClose(IbcChannelCloseMsg),

    PacketReceive(IbcPacketReceiveMsg),
    PacketAcknowledgement(IbcPacketAckMsg),
    PacketTimeout(IbcPacketTimeoutMsg),
}

pub trait Ibc: Module<ExecT = IbcMsg, QueryT = MockIbcQuery, SudoT = IbcPacketRelayingMsg> {}

pub type IbcAcceptingModule = AcceptingModule<IbcMsg, MockIbcQuery, IbcPacketRelayingMsg>;

impl Ibc for IbcAcceptingModule {}

pub type IbcFailingModule = FailingModule<IbcMsg, MockIbcQuery, IbcPacketRelayingMsg>;

impl Ibc for IbcFailingModule {}

#[cfg(test)]
mod test;
