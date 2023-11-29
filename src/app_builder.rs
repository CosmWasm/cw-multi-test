//! Implementation of the builder for [App].

use crate::{
    App, Bank, BankKeeper, Distribution, DistributionKeeper, FailingModule, Gov, GovFailingModule,
    Ibc, IbcFailingModule, Module, Router, StakeKeeper, Staking, Wasm, WasmKeeper,
};
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{Api, BlockInfo, CustomQuery, Empty, Storage};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

/// This is essential to create a custom app with custom module.
///
/// # Example
///
/// ```
/// # use cosmwasm_std::Empty;
/// # use cw_multi_test::{BasicAppBuilder, FailingModule, Module, no_init};
/// # type MyHandler = FailingModule<Empty, Empty, Empty>;
/// # type MyExecC = Empty;
/// # type MyQueryC = Empty;
///
/// let mut app = BasicAppBuilder::<MyExecC, MyQueryC>::new_custom()
///                   .with_custom(MyHandler::default())
///                   .build(no_init);
/// ```
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
>;

/// Utility to build [App] in stages.
/// When particular properties are not explicitly set, then default values are used.
pub struct AppBuilder<Bank, Api, Storage, Custom, Wasm, Staking, Distr, Ibc, Gov> {
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
    >
where
    ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
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
        }
    }
}

impl<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT>
    AppBuilder<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT>
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
    ) -> AppBuilder<BankT, ApiT, StorageT, CustomT, NewWasm, StakingT, DistrT, IbcT, GovT> {
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
        }
    }

    /// Overwrites the default bank interface.
    pub fn with_bank<NewBank: Bank>(
        self,
        bank: NewBank,
    ) -> AppBuilder<NewBank, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT> {
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
        }
    }

    /// Overwrites the default api interface.
    pub fn with_api<NewApi: Api>(
        self,
        api: NewApi,
    ) -> AppBuilder<BankT, NewApi, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT> {
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
        }
    }

    /// Overwrites the default storage interface.
    pub fn with_storage<NewStorage: Storage>(
        self,
        storage: NewStorage,
    ) -> AppBuilder<BankT, ApiT, NewStorage, CustomT, WasmT, StakingT, DistrT, IbcT, GovT> {
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
        }
    }

    /// Overwrites the default custom messages handler.
    ///
    /// At this point it is needed that new custom implements some `Module` trait, but it doesn't need
    /// to be bound to ExecC or QueryC yet - as those may change. The cross-components validation is
    /// done on final building.
    pub fn with_custom<NewCustom: Module>(
        self,
        custom: NewCustom,
    ) -> AppBuilder<BankT, ApiT, StorageT, NewCustom, WasmT, StakingT, DistrT, IbcT, GovT> {
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
        }
    }

    /// Overwrites the default staking interface.
    pub fn with_staking<NewStaking: Staking>(
        self,
        staking: NewStaking,
    ) -> AppBuilder<BankT, ApiT, StorageT, CustomT, WasmT, NewStaking, DistrT, IbcT, GovT> {
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
        }
    }

    /// Overwrites the default distribution interface.
    pub fn with_distribution<NewDistribution: Distribution>(
        self,
        distribution: NewDistribution,
    ) -> AppBuilder<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, NewDistribution, IbcT, GovT>
    {
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
    ) -> AppBuilder<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, NewIbc, GovT> {
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
        }
    }

    /// Overwrites the default gov interface.
    pub fn with_gov<NewGov: Gov>(
        self,
        gov: NewGov,
    ) -> AppBuilder<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, NewGov> {
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
        }
    }

    /// Overwrites the initial block.
    pub fn with_block(mut self, block: BlockInfo) -> Self {
        self.block = block;
        self
    }

    /// Builds final `App`. At this point all components type have to be properly related to each
    /// other. If there are some generics related compilation errors, make sure that all components
    /// are properly relating to each other.
    pub fn build<F>(
        self,
        init_fn: F,
    ) -> App<BankT, ApiT, StorageT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT>
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
        F: FnOnce(
            &mut Router<BankT, CustomT, WasmT, StakingT, DistrT, IbcT, GovT>,
            &dyn Api,
            &mut dyn Storage,
        ),
    {
        let router = Router {
            wasm: self.wasm,
            bank: self.bank,
            custom: self.custom,
            staking: self.staking,
            distribution: self.distribution,
            ibc: self.ibc,
            gov: self.gov,
        };

        let mut app = App {
            router,
            api: self.api,
            block: self.block,
            storage: self.storage,
        };
        app.init_modules(init_fn);
        app
    }
}
