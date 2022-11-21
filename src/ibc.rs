use cosmwasm_std::{Empty, IbcMsg, IbcQuery};

use crate::{FailingModule, Module};

pub trait Ibc: Module<ExecT = IbcMsg, QueryT = IbcQuery, SudoT = Empty> {}

impl Ibc for FailingModule<IbcMsg, IbcQuery, Empty> {}

#[cfg(test)]
mod test {
    use cosmwasm_std::{Addr, Binary, Empty, IbcMsg, IbcQuery};

    use crate::test_helpers::contracts::stargate::{contract, ExecMsg};
    use crate::{App, AppBuilder, AppResponse, Executor, Module};

    use super::Ibc;

    struct AcceptingModule;

    impl Module for AcceptingModule {
        type ExecT = IbcMsg;
        type QueryT = IbcQuery;
        type SudoT = Empty;

        fn execute<ExecC, QueryC>(
            &self,
            _api: &dyn cosmwasm_std::Api,
            _storage: &mut dyn cosmwasm_std::Storage,
            _router: &dyn crate::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
            _block: &cosmwasm_std::BlockInfo,
            _sender: cosmwasm_std::Addr,
            _msg: Self::ExecT,
        ) -> anyhow::Result<crate::AppResponse>
        where
            ExecC: std::fmt::Debug
                + Clone
                + PartialEq
                + schemars::JsonSchema
                + serde::de::DeserializeOwned
                + 'static,
            QueryC: cosmwasm_std::CustomQuery + serde::de::DeserializeOwned + 'static,
        {
            Ok(AppResponse::default())
        }

        fn sudo<ExecC, QueryC>(
            &self,
            _api: &dyn cosmwasm_std::Api,
            _storage: &mut dyn cosmwasm_std::Storage,
            _router: &dyn crate::CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
            _block: &cosmwasm_std::BlockInfo,
            _msg: Self::SudoT,
        ) -> anyhow::Result<crate::AppResponse>
        where
            ExecC: std::fmt::Debug
                + Clone
                + PartialEq
                + schemars::JsonSchema
                + serde::de::DeserializeOwned
                + 'static,
            QueryC: cosmwasm_std::CustomQuery + serde::de::DeserializeOwned + 'static,
        {
            Ok(AppResponse::default())
        }

        fn query(
            &self,
            _api: &dyn cosmwasm_std::Api,
            _storage: &dyn cosmwasm_std::Storage,
            _querier: &dyn cosmwasm_std::Querier,
            _block: &cosmwasm_std::BlockInfo,
            _request: Self::QueryT,
        ) -> anyhow::Result<cosmwasm_std::Binary> {
            Ok(Binary::default())
        }
    }

    impl Ibc for AcceptingModule {}

    #[test]
    fn default_ibc() {
        let mut app = App::default();
        let code = app.store_code(contract());
        let contract = app
            .instantiate_contract(
                code,
                Addr::unchecked("owner"),
                &Empty {},
                &[],
                "contract",
                None,
            )
            .unwrap();

        app.execute_contract(Addr::unchecked("owner"), contract, &ExecMsg::Ibc {}, &[])
            .unwrap_err();
    }

    #[test]
    fn subsituting_ibc() {
        let mut app = AppBuilder::new()
            .with_ibc(AcceptingModule)
            .build(|_, _, _| ());
        let code = app.store_code(contract());
        let contract = app
            .instantiate_contract(
                code,
                Addr::unchecked("owner"),
                &Empty {},
                &[],
                "contract",
                None,
            )
            .unwrap();

        app.execute_contract(Addr::unchecked("owner"), contract, &ExecMsg::Ibc {}, &[])
            .unwrap();
    }
}
