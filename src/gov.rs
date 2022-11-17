use cosmwasm_std::{Empty, GovMsg};

use crate::{FailingModule, Module};

pub trait Gov: Module<ExecT = GovMsg, QueryT = Empty, SudoT = Empty> {}

impl Gov for FailingModule<GovMsg, Empty, Empty> {}
