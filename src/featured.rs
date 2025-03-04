//! # Definitions enabled or disabled by crate's features

#[cfg(feature = "stargate")]
pub use cosmwasm_std::GovMsg;

#[cfg(not(feature = "stargate"))]
pub use cosmwasm_std::Empty as GovMsg;

#[cfg(feature = "staking")]
pub mod staking {
    pub use crate::staking::{
        Distribution, DistributionKeeper, StakeKeeper, Staking, StakingInfo, StakingSudo,
    };
}

#[cfg(not(feature = "staking"))]
pub mod staking {
    use crate::error::AnyResult;
    use crate::{AppResponse, CosmosRouter, FailingModule, Module};
    use cosmwasm_std::{Api, BlockInfo, CustomMsg, CustomQuery, Empty, Storage};

    /// Empty staking privileged action definition.
    pub enum StakingSudo {}

    /// Empty general staking parameters.
    pub struct StakingInfo {}

    /// A trait defining a behavior of the stake keeper.
    pub trait Staking: Module<ExecT = Empty, QueryT = Empty, SudoT = Empty> {
        /// This is no-op for dummy staking module.
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

    /// A structure representing a default stake keeper, always failing module.
    pub type StakeKeeper = FailingModule<Empty, Empty, Empty>;

    impl Staking for StakeKeeper {}

    /// A trait defining a behavior of the distribution keeper.
    pub trait Distribution: Module<ExecT = Empty, QueryT = Empty, SudoT = Empty> {}

    /// A structure representing a default distribution keeper, always failing module.
    pub type DistributionKeeper = FailingModule<Empty, Empty, Empty>;

    impl Distribution for DistributionKeeper {}
}
