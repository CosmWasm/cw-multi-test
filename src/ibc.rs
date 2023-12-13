use crate::{AcceptingModule, FailingModule, Module};
use cosmwasm_std::{Empty, IbcMsg, IbcQuery};
///Manages Inter-Blockchain Communication (IBC) functionalities.
///This trait is critical for testing contracts that involve cross-chain interactions,
///reflecting the interconnected nature of the Cosmos ecosystem.
pub trait Ibc: Module<ExecT = IbcMsg, QueryT = IbcQuery, SudoT = Empty> {}
/// Ideal for testing contracts that involve IBC, this module is designed to successfully
/// handle cross-chain messages. It's key for ensuring that your contract can smoothly interact
/// with other blockchains in the Cosmos network.
pub type IbcAcceptingModule = AcceptingModule<IbcMsg, IbcQuery, Empty>;

impl Ibc for IbcAcceptingModule {}
/// Use this to test how your contract deals with problematic IBC scenarios.
/// It's a module that deliberately fails in handling IBC messages, allowing you
/// to check how your contract behaves in less-than-ideal cross-chain communication situations.
pub type IbcFailingModule = FailingModule<IbcMsg, IbcQuery, Empty>;

impl Ibc for IbcFailingModule {}
