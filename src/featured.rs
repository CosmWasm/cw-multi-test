//! # Definitions enabled or disabled by crate's features

#[cfg(feature = "stargate")]
pub use cosmwasm_std::GovMsg;

#[cfg(not(feature = "stargate"))]
pub use cosmwasm_std::Empty as GovMsg;

#[cfg(feature = "staking")]
pub mod staking {
    pub use crate::staking::{Distribution, DistributionKeeper, StakeKeeper, Staking, StakingSudo};
}

#[cfg(not(feature = "staking"))]
pub mod staking {
    use crate::error::AnyResult;
    use crate::{AppResponse, CosmosRouter, FailingModule, Module};
    use cosmwasm_std::{Api, BlockInfo, CustomMsg, CustomQuery, Empty, Storage};

    pub enum StakingSudo {}

    pub trait Staking: Module<ExecT = Empty, QueryT = Empty, SudoT = Empty> {
        fn process_queue<ExecC: CustomMsg, QueryC: CustomQuery>(
            &self,
            _api: &dyn Api,
            _storage: &mut dyn Storage,
            _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
            _block: &BlockInfo,
        ) -> AnyResult<AppResponse> {
            Ok(AppResponse::default())
        }
    }

    pub type StakeKeeper = FailingModule<Empty, Empty, Empty>;

    impl Staking for StakeKeeper {}

    pub trait Distribution: Module<ExecT = Empty, QueryT = Empty, SudoT = Empty> {}

    pub type DistributionKeeper = FailingModule<Empty, Empty, Empty>;

    impl Distribution for DistributionKeeper {}
}
