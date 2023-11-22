use crate::{AcceptingModule, FailingModule, Module};
use cosmwasm_std::Empty;

#[cfg(feature = "stargate")]
type GovMsg = cosmwasm_std::GovMsg;

#[cfg(not(feature = "stargate"))]
type GovMsg = cosmwasm_std::Empty;

pub trait Gov: Module<ExecT = GovMsg, QueryT = Empty, SudoT = Empty> {}

pub type GovAcceptingModule = AcceptingModule<GovMsg, Empty, Empty>;

impl Gov for GovAcceptingModule {}

pub type GovFailingModule = FailingModule<GovMsg, Empty, Empty>;

impl Gov for GovFailingModule {}
