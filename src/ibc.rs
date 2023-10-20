use crate::{AcceptingModule, FailingModule, Module};
use cosmwasm_std::{Empty, IbcMsg, IbcQuery};

pub trait Ibc: Module<ExecT = IbcMsg, QueryT = IbcQuery, SudoT = Empty> {}

pub type IbcAcceptingModule = AcceptingModule<IbcMsg, IbcQuery, Empty>;

impl Ibc for IbcAcceptingModule {}

pub type IbcFailingModule = FailingModule<IbcMsg, IbcQuery, Empty>;

impl Ibc for IbcFailingModule {}
