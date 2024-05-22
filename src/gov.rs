use crate::{AcceptingModule, FailingModule, Module};
use cosmwasm_std::Empty;
#[cfg(not(feature = "stargate"))]
use cosmwasm_std::Empty as GovMsg;
#[cfg(feature = "stargate")]
use cosmwasm_std::GovMsg;

/// Handles governance-related operations within the test environment.
/// This trait is essential for testing contracts that interact with governance mechanisms,
/// simulating proposals, voting, and other governance activities.
pub trait Gov: Module<ExecT = GovMsg, QueryT = Empty, SudoT = Empty> {}

/// A type alias for a module that accepts governance-related interactions.
/// It's used in scenarios where you need to test how your contract interacts
/// with governance processes and messages.
pub type GovAcceptingModule = AcceptingModule<GovMsg, Empty, Empty>;

impl Gov for GovAcceptingModule {}

/// This type alias represents a module designed to fail in response to governance operations.
/// It's useful for testing how contracts behave when governance actions do not proceed as expected.
pub type GovFailingModule = FailingModule<GovMsg, Empty, Empty>;

impl Gov for GovFailingModule {}
