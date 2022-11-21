use cosmwasm_std::Empty;

use crate::{FailingModule, Module};

#[cfg(feature = "stargate")]
compile_error!("Intended for use only with `stargate` feature disabled!");

pub trait Gov: Module<ExecT = Empty, QueryT = Empty, SudoT = Empty> {}

impl Gov for FailingModule<Empty, Empty, Empty> {}

pub type FailingGovKeeper = FailingModule<Empty, Empty, Empty>;
