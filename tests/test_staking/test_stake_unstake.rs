use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{BlockInfo, Decimal, Validator};
use cw_multi_test::{no_init, App, AppBuilder, IntoBech32, StakingInfo};

#[test]
fn stake_unstake_should_work() {
    let validator_addr = "oper1".into_bech32();
    let validator_interest_rate: Decimal = Decimal::percent(10);
    let validator_commission: Decimal = Decimal::percent(10);

    let valoper1 = Validator::new(
        validator_addr.to_string(),
        validator_commission,
        Decimal::percent(100),
        Decimal::percent(1),
    );

    let block = mock_env().block;

    let app_builder = AppBuilder::default();
    let mut app = app_builder.build(|router, api, storage| {
        router
            .staking
            .setup(
                storage,
                StakingInfo {
                    bonded_denom: "TOKEN".to_string(),
                    unbonding_time: 60,
                    apr: validator_interest_rate,
                },
            )
            .unwrap();
        router
            .staking
            .add_validator(api, storage, &block, valoper1)
            .unwrap();
    });
}
