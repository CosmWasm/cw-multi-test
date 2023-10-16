use crate::test_app_builder::MyKeeper;
use cosmwasm_std::{Addr, Coin, StakingMsg, StakingQuery};
use cw_multi_test::{AppBuilder, Executor, Staking, StakingSudo};

type MyStakeKeeper = MyKeeper<StakingMsg, StakingQuery, StakingSudo>;

impl Staking for MyStakeKeeper {}

const EXECUTE_MSG: &str = "staking execute called";
const QUERY_MSG: &str = "staking query called";
const SUDO_MSG: &str = "staking sudo called";

#[test]
#[cfg(feature = "staking")]
fn building_app_with_custom_staking_should_work() {
    // build custom stake keeper
    let stake_keeper = MyStakeKeeper::new(EXECUTE_MSG, QUERY_MSG, SUDO_MSG);

    // build the application with custom stake keeper
    let app_builder = AppBuilder::default();
    let mut app = app_builder.with_staking(stake_keeper).build(|_, _, _| {});

    // prepare additional input data
    let validator = Addr::unchecked("recipient");

    // executing staking message should return an error defined in custom keeper
    assert_eq!(
        EXECUTE_MSG,
        app.execute(
            Addr::unchecked("sender"),
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
