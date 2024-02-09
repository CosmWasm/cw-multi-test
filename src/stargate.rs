use crate::{AcceptingModule, FailingModule, Module};
use cosmwasm_std::{AnyMsg, Empty, GrpcQuery};

/// Interface to module handling any messages and queries.
pub trait Stargate: Module<ExecT = AnyMsg, QueryT = GrpcQuery, SudoT = Empty> {}

/// Always accepting stargate module.
pub type StargateAcceptingModule = AcceptingModule<AnyMsg, GrpcQuery, Empty>;

impl Stargate for StargateAcceptingModule {}

/// Always accepting stargate module.
pub type StargateFailingModule = FailingModule<AnyMsg, GrpcQuery, Empty>;

impl Stargate for StargateFailingModule {}
