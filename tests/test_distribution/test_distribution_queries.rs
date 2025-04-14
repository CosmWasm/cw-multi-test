use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{coin, Decimal, Validator};
use cosmwasm_std::{DistributionMsg, StakingMsg};
use cw_multi_test::{AppBuilder, Executor, IntoAddr, StakingInfo};

#[test]
fn querying_withdraw_address_should_work() {
    const BONDED_DENOM: &str = "stake"; // Denominator of the staking token.
    const UNBONDING_TIME: u64 = 60; // Time between unbonding and receiving tokens back (in seconds).
    const DELEGATION_AMOUNT: u128 = 100; // Amount of tokens to be Delegated.
    const INITIAL_AMOUNT: u128 = 1000; // Initial amount of tokens of delegator.

    // Prepare delegator address.
    let delegator_addr = "delegator".into_addr();
    // Prepare address for staking rewards.
    let withdraw_address = "rewards".into_addr();
    // Prepare validator address.
    let validator_addr = "valoper".into_addr();

    // Configure a new validator.
    let valoper = Validator::new(
        validator_addr.to_string(),
        Decimal::percent(10),
        Decimal::percent(90),
        Decimal::percent(1),
    );

    // Prepare the blockchain configuration.
    let block = mock_env().block;
    let mut app = AppBuilder::default().build(|router, api, storage| {
        // Set the initial balance for the delegator.
        router
            .bank
            .init_balance(
                storage,
                &delegator_addr,
                vec![coin(INITIAL_AMOUNT, BONDED_DENOM)],
            )
            .unwrap();
        // Setup staking parameters.
        router
            .staking
            .setup(
                storage,
                StakingInfo {
                    bonded_denom: BONDED_DENOM.to_string(),
                    unbonding_time: UNBONDING_TIME,
                    apr: Decimal::percent(10),
                },
            )
            .unwrap();
        // Add a validator.
        router
            .staking
            .add_validator(api, storage, &block, valoper)
            .unwrap();
    });

    // Delegate tokens to validator.
    app.execute(
        delegator_addr.clone(),
        StakingMsg::Delegate {
            validator: validator_addr.to_string(),
            amount: coin(DELEGATION_AMOUNT, BONDED_DENOM),
        }
        .into(),
    )
    .unwrap();

    // Set withdraw address for rewards from staking.
    app.execute(
        delegator_addr.clone(),
        DistributionMsg::SetWithdrawAddress {
            address: withdraw_address.to_string(),
        }
        .into(),
    )
    .unwrap();

    let address = app
        .wrap()
        .query_delegator_withdraw_address(delegator_addr)
        .unwrap();

    assert_eq!(withdraw_address.as_str(), address.as_str());
}
