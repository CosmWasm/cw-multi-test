use crate::{AcceptingModule, FailingModule, Module};
use cosmwasm_std::{AnyMsg, Empty, GrpcQuery};

/// Interface to module handling any messages and queries.
pub trait Anygate: Module<ExecT = AnyMsg, QueryT = GrpcQuery, SudoT = Empty> {}

/// Always accepting module for [Anygate].
pub type AnygateAcceptingModule = AcceptingModule<AnyMsg, GrpcQuery, Empty>;

impl Anygate for AnygateAcceptingModule {}

impl AnygateAcceptingModule {}

/// Always failing module for [Anygate].
pub type AnygateFailingModule = FailingModule<AnyMsg, GrpcQuery, Empty>;

impl Anygate for AnygateFailingModule {}
