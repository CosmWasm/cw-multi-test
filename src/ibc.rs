use cosmwasm_std::{Empty, IbcMsg, IbcQuery};

use crate::{FailingModule, Module};

pub trait Ibc: Module<ExecT = IbcMsg, QueryT = IbcQuery, SudoT = Empty> {}

impl Ibc for FailingModule<IbcMsg, IbcQuery, Empty> {}
