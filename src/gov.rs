use crate::featured::GovMsg;
use crate::{AcceptingModule, FailingModule, Module};
use cosmwasm_std::Empty;

/// This trait implements the interface of the governance module.
pub trait Gov: Module<ExecT = GovMsg, QueryT = Empty, SudoT = Empty> {}

/// Implementation of the always accepting governance module.
pub type GovAcceptingModule = AcceptingModule<GovMsg, Empty, Empty>;

impl Gov for GovAcceptingModule {}

/// Implementation of the always failing governance module.
pub type GovFailingModule = FailingModule<GovMsg, Empty, Empty>;

impl Gov for GovFailingModule {}
