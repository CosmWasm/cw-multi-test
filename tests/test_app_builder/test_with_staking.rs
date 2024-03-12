use crate::test_app_builder::MyKeeper;
use cosmwasm_std::{Api, BlockInfo, Coin, CustomQuery, StakingMsg, StakingQuery, Storage};
use cw_multi_test::error::AnyResult;
use cw_multi_test::{
    no_init, AppBuilder, AppResponse, CosmosRouter, Executor, Staking, StakingSudo,
};

type MyStakeKeeper = MyKeeper<StakingMsg, StakingQuery, StakingSudo>;

impl Staking for MyStakeKeeper {
    fn process_queue<ExecC, QueryC: CustomQuery>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
    ) -> AnyResult<AppResponse> {
        let _ = (api, storage, router, block);
        todo!()
    }
}

const EXECUTE_MSG: &str = "staking execute called";
const QUERY_MSG: &str = "staking query called";
const SUDO_MSG: &str = "staking sudo called";

#[test]
fn building_app_with_custom_staking_should_work() {
    // build custom stake keeper
    let stake_keeper = MyStakeKeeper::new(EXECUTE_MSG, QUERY_MSG, SUDO_MSG);

    // build the application with custom stake keeper
    let app_builder = AppBuilder::default();
    let mut app = app_builder.with_staking(stake_keeper).build(no_init);

    // prepare additional input data
    let validator = app.api().addr_make("recipient");

    // executing staking message should return an error defined in custom keeper
    assert_eq!(
        EXECUTE_MSG,
        app.execute(
            app.api().addr_make("sender"),
            StakingMsg::Delegate {
                validator: validator.clone().into(),
                amount: Coin::new(1, "eth"),
            }
            .into(),
        )
        .unwrap_err()
        .to_string()
    );

    // executing staking sudo should return an error defined in custom keeper
    assert_eq!(
        SUDO_MSG,
        app.sudo(
            StakingSudo::Slash {
                validator: validator.into(),
                percentage: Default::default(),
            }
            .into()
        )
        .unwrap_err()
        .to_string()
    );

    // executing staking query should return an error defined in custom keeper
    assert_eq!(
        format!("Generic error: Querier contract error: {}", QUERY_MSG),
        app.wrap().query_all_validators().unwrap_err().to_string()
    );
}
