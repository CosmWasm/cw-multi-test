use crate::{AcceptingModule, FailingModule, Module};
use cosmwasm_std::{Empty, IbcMsg, IbcQuery};

/// This trait implements the interface for IBC functionalities.
pub trait Ibc: Module<ExecT = IbcMsg, QueryT = IbcQuery, SudoT = Empty> {}

/// Implementation of the always accepting IBC module.
pub type IbcAcceptingModule = AcceptingModule<IbcMsg, IbcQuery, Empty>;

impl Ibc for IbcAcceptingModule {}

/// implementation of the always failing IBC module.
pub type IbcFailingModule = FailingModule<IbcMsg, IbcQuery, Empty>;

impl Ibc for IbcFailingModule {}
