use cosmwasm_std::Empty;

use crate::module::{FailingModule, Module};

#[cfg(feature = "stargate")]
compile_error!("Intended for use only with `stargate` feature disabled!");

pub trait Ibc: Module<ExecT = Empty, QueryT = Empty, SudoT = Empty> {}

impl Ibc for FailingModule<Empty, Empty, Empty> {}

pub type FailingIbcKeeper = FailingModule<Empty, Empty, Empty>;
