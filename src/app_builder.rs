//! AppBuilder helps you set up your test blockchain environment step by step [App].

use crate::featured::staking::{Distribution, DistributionKeeper, StakeKeeper, Staking};
use crate::{
    App, Bank, BankKeeper, FailingModule, Gov, GovFailingModule, Ibc, IbcFailingModule, Module,
    Router, Stargate, StargateFailing, Wasm, WasmKeeper,
};
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{Api, BlockInfo, CustomMsg, CustomQuery, Empty, Storage};
use serde::de::DeserializeOwned;
use std::fmt::Debug;

/// This is essential to create a custom app with custom module.
///
/// # Example
///
/// ```
/// # use cosmwasm_std::Empty;
/// # use cw_multi_test::{no_init, BasicAppBuilder, FailingModule, Module};
/// # type MyHandler = FailingModule<Empty, Empty, Empty>;
/// # type MyExecC = Empty;
/// # type MyQueryC = Empty;
///
/// let mut app = BasicAppBuilder::<MyExecC, MyQueryC>::new_custom()
///                   .with_custom(MyHandler::default())
///                   .build(no_init);
/// ```
/// This type alias is crucial for constructing a custom app with specific modules.
/// It provides a streamlined approach to building and configuring an App tailored to
/// particular testing needs or scenarios.
pub type BasicAppBuilder<ExecC, QueryC> = AppBuilder<
    BankKeeper,
    MockApi,
    MockStorage,
    FailingModule<ExecC, QueryC, Empty>,
    WasmKeeper<ExecC, QueryC>,
    StakeKeeper,
    DistributionKeeper,
    IbcFailingModule,
    GovFailingModule,
    StargateFailing,
>;

/// Utility to build [App] in stages.
/// When particular properties are not explicitly set, then default values are used.
pub struct AppBuilder<Bank, Api, Storage, Custom, Wasm, Staking, Distr, Ibc, Gov, Stargate> {
    api: Api,
    block: BlockInfo,
    storage: Storage,
    bank: Bank,
    wasm: Wasm,
    custom: Custom,
    staking: Staking,
    distribution: Distr,
    ibc: Ibc,
    gov: Gov,
    stargate: Stargate,
}

impl Default
    for AppBuilder<
        BankKeeper,
        MockApi,
        MockStorage,
        FailingModule<Empty, Empty, Empty>,
        WasmKeeper<Empty, Empty>,
        StakeKeeper,
        DistributionKeeper,
        IbcFailingModule,
        GovFailingModule,
        StargateFailing,
    >
{
    fn default() -> Self {
        Self::new()
    }
}

impl
    AppBuilder<
        BankKeeper,
        MockApi,
        MockStorage,
        FailingModule<Empty, Empty, Empty>,
        WasmKeeper<Empty, Empty>,
        StakeKeeper,
        DistributionKeeper,
        IbcFailingModule,
        GovFailingModule,
        StargateFailing,
    >
{
    /// Creates builder with default components working with empty exec and query messages.
    pub fn new() -> Self {
        AppBuilder {
            api: MockApi::default(),
            block: mock_env().block,
            storage: MockStorage::new(),
            bank: BankKeeper::new(),
            wasm: WasmKeeper::new(),
            custom: FailingModule::new(),
            staking: StakeKeeper::new(),
            distribution: DistributionKeeper::new(),
            ibc: IbcFailingModule::new(),
            gov: GovFailingModule::new(),
            stargate: StargateFailing,
        }
    }
}

impl<ExecC, QueryC>
    AppBuilder<
        BankKeeper,
        MockApi,
        MockStorage,
        FailingModule<ExecC, QueryC, Empty>,
        WasmKeeper<ExecC, QueryC>,
        StakeKeeper,
        DistributionKeeper,
        IbcFailingModule,
        GovFailingModule,
        StargateFailing,
    >
where
    ExecC: CustomMsg + DeserializeOwned + 'static,
    QueryC: Debug + CustomQuery + DeserializeOwned + 'static,
{
    /// Creates builder with default components designed to work with custom exec and query
    /// messages.
    pub fn new_custom() -> Self {
        AppBuilder {
            api: MockApi::default(),
            block: mock_env().block,
            storage: MockStorage::new(),
            bank: BankKeeper::new(),
            wasm: WasmKeeper::new(),
            custom: FailingModule::new(),
            staking: StakeKeeper::new(),
            distribution: DistributionKeeper::new(),
            ibc: IbcFailingModule::new(),
            gov: GovFailingModule::new(),
            stargate: StargateFailing,
        }
    }
}

impl<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
    AppBuilder<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
where
    CustomT: Module,
    WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
{
    /// Overwrites the default wasm executor.
    ///
    /// At this point it is needed that new wasm implements some `Wasm` trait, but it doesn't need
    /// to be bound to Bank or Custom yet - as those may change. The cross-components validation is
    /// done on final building.
    pub fn with_wasm<NewWasm: Wasm<CustomT::ExecT, CustomT::QueryT>>(
        self,
        wasm: NewWasm,
    ) -> AppBuilder<BankT, ApiT, StorageT, CustomT, NewWasm, StakingT, DistrT, IbcT, GovT, StargateT>
    {
        let AppBuilder {
            bank,
            api,
            storage,
            custom,
            block,
            staking,
            distribution,
            ibc,
            gov,
            stargate,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            distribution,
            ibc,
            gov,
            stargate,
        }
    }

    /// Overwrites the default bank interface.
    pub fn with_bank<NewBank: Bank>(
        self,
        bank: NewBank,
    ) -> AppBuilder<NewBank, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
    {
        let AppBuilder {
            wasm,
            api,
            storage,
            custom,
            block,
            staking,
            distribution,
            ibc,
            gov,
            stargate,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            distribution,
            ibc,
            gov,
            stargate,
        }
    }

    /// Overwrites the default api interface.
    pub fn with_api<NewApi: Api>(
        self,
        api: NewApi,
    ) -> AppBuilder<BankT, NewApi, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
    {
        let AppBuilder {
            wasm,
            bank,
            storage,
            custom,
            block,
            staking,
            distribution,
            ibc,
            gov,
            stargate,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            distribution,
            ibc,
            gov,
            stargate,
        }
    }

    /// Overwrites the default storage interface.
    pub fn with_storage<NewStorage: Storage>(
        self,
        storage: NewStorage,
    ) -> AppBuilder<BankT, ApiT, NewStorage, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
    {
        let AppBuilder {
            wasm,
            api,
            bank,
            custom,
            block,
            staking,
            distribution,
            ibc,
            gov,
            stargate,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            distribution,
            ibc,
            gov,
            stargate,
        }
    }

    /// Overwrites the default handler for custom messages.
    ///
    /// At this point it is needed that new custom implements some `Module` trait, but it doesn't need
    /// to be bound to ExecC or QueryC yet - as those may change. The cross-components validation is
    /// done on final building.
    pub fn with_custom<NewCustom: Module>(
        self,
        custom: NewCustom,
    ) -> AppBuilder<BankT, ApiT, StorageT, NewCustom, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
    {
        let AppBuilder {
            wasm,
            bank,
            api,
            storage,
            block,
            staking,
            distribution,
            ibc,
            gov,
            stargate,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            distribution,
            ibc,
            gov,
            stargate,
        }
    }

    /// Overwrites the default staking interface.
    pub fn with_staking<NewStaking: Staking>(
        self,
        staking: NewStaking,
    ) -> AppBuilder<BankT, ApiT, StorageT, CustomT, WasmT, NewStaking, DistrT, IbcT, GovT, StargateT>
    {
        let AppBuilder {
            wasm,
            api,
            storage,
            custom,
            block,
            bank,
            distribution,
            ibc,
            gov,
            stargate,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            distribution,
            ibc,
            gov,
            stargate,
        }
    }

    /// Overwrites the default distribution interface.
    pub fn with_distribution<NewDistribution: Distribution>(
        self,
        distribution: NewDistribution,
    ) -> AppBuilder<
        BankT,
        ApiT,
        StorageT,
        CustomT,
        WasmT,
        StakingT,
        NewDistribution,
        IbcT,
        GovT,
        StargateT,
    > {
        let AppBuilder {
            wasm,
            api,
            storage,
            custom,
            block,
            staking,
            bank,
            ibc,
            gov,
            stargate,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            distribution,
            ibc,
            gov,
            stargate,
        }
    }

    /// Overwrites the default ibc interface.
    ///
    /// If you wish to simply ignore/drop all returned IBC Messages,
    /// you can use the `IbcAcceptingModule` type:
    /// ```text
    /// builder.with_ibc(IbcAcceptingModule::new())
    /// ```
    pub fn with_ibc<NewIbc: Ibc>(
        self,
        ibc: NewIbc,
    ) -> AppBuilder<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, NewIbc, GovT, StargateT>
    {
        let AppBuilder {
            wasm,
            api,
            storage,
            custom,
            block,
            staking,
            bank,
            distribution,
            gov,
            stargate,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            stargate,
            distribution,
            ibc,
            gov,
        }
    }

    /// Overwrites the default gov interface.
    pub fn with_gov<NewGov: Gov>(
        self,
        gov: NewGov,
    ) -> AppBuilder<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, NewGov, StargateT>
    {
        let AppBuilder {
            wasm,
            api,
            storage,
            custom,
            block,
            staking,
            bank,
            distribution,
            ibc,
            stargate,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            distribution,
            ibc,
            gov,
            stargate,
        }
    }

    /// Overwrites the default stargate interface.
    pub fn with_stargate<NewStargate: Stargate>(
        self,
        stargate: NewStargate,
    ) -> AppBuilder<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, NewStargate>
    {
        let AppBuilder {
            wasm,
            api,
            storage,
            custom,
            block,
            staking,
            bank,
            distribution,
            ibc,
            gov,
            ..
        } = self;

        AppBuilder {
            api,
            block,
            storage,
            bank,
            wasm,
            custom,
            staking,
            distribution,
            ibc,
            gov,
            stargate,
        }
    }

    /// Overwrites the initial block.
    pub fn with_block(mut self, block: BlockInfo) -> Self {
        self.block = block;
        self
    }

    /// Builds the final [App] with initialization.
    ///
    /// At this point all component types have to be properly related to each other.
    pub fn build<F>(
        self,
        init_fn: F,
    ) -> App<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>
    where
        BankT: Bank,
        ApiT: Api,
        StorageT: Storage,
        CustomT: Module,
        WasmT: Wasm<CustomT::ExecT, CustomT::QueryT>,
        StakingT: Staking,
        DistrT: Distribution,
        IbcT: Ibc,
        GovT: Gov,
        StargateT: Stargate,
        F: FnOnce(
            &mut Router<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT, StargateT>,
            &ApiT,
            &mut dyn Storage,
        ),
    {
        // build the final application
        let mut app = App {
            router: Router {
                wasm: self.wasm,
                bank: self.bank,
                custom: self.custom,
                staking: self.staking,
                distribution: self.distribution,
                ibc: self.ibc,
                gov: self.gov,
                stargate: self.stargate,
            },
            api: self.api,
            block: self.block,
            storage: self.storage,
        };
        // execute initialization provided by the caller
        app.init_modules(init_fn);
        // return already initialized application
        app
    }
}
