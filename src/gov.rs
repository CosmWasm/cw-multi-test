use crate::{FailingModule, Module};
use cosmwasm_std::{Empty, GovMsg};

pub trait Gov: Module<ExecT = GovMsg, QueryT = Empty, SudoT = Empty> {}

impl Gov for FailingModule<GovMsg, Empty, Empty> {}
