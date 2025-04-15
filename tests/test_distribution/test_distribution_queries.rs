use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{coin, Decimal, Decimal256, DistributionMsg, StakingMsg, Uint128, Validator};
use cw_multi_test::{App, AppBuilder, Executor, IntoAddr, StakingInfo};

/// Denominator of the staking token.
const BONDED_DENOM: &str = "stake";

/// Initial amount of tokens for each delegator.
const INITIAL_BALANCE: u128 = 1000;

/// Time between unbonding and receiving tokens back, in seconds.
const UNBONDING_TIME: u64 = 60;

/// Amount of tokens to be delegated.
const DELEGATION_AMOUNT: u128 = 100;

const SECONDS_IN_YEAR: u64 = 365 * 24 * 60 * 60;

// Test for querying withdraw addresses.
#[test]
fn querying_withdraw_address_should_work() {
    // Prepare the delegator address.
    let delegator_addr = "delegator".into_addr();
    // Prepare the address for staking rewards.
    let withdraw_address = "rewards".into_addr();
    // Create a chain with default settings.
    let mut app = App::default();

    // Before changing withdraw address, the queried one should be equal to the delegator address.
    assert_eq!(
        delegator_addr.as_str(),
        app.wrap()
            .query_delegator_withdraw_address(delegator_addr.clone())
            .unwrap()
            .as_str()
    );

    // Change withdraw address for specified delegator.
    app.execute(
        delegator_addr.clone(),
        DistributionMsg::SetWithdrawAddress {
            address: withdraw_address.to_string(),
        }
        .into(),
    )
    .unwrap();

    // Queried withdraw address should be equal to the one set by delegator.
    assert_eq!(
        withdraw_address.as_str(),
        app.wrap()
            .query_delegator_withdraw_address(delegator_addr.clone())
            .unwrap()
            .as_str()
    );

    // Change withdraw address to delegator address (remove withdraw address).
    app.execute(
        delegator_addr.clone(),
        DistributionMsg::SetWithdrawAddress {
            address: delegator_addr.clone().to_string(),
        }
        .into(),
    )
    .unwrap();

    // The queried address should be equal to the delegator address.
    assert_eq!(
        delegator_addr.as_str(),
        app.wrap()
            .query_delegator_withdraw_address(delegator_addr.clone())
            .unwrap()
            .as_str()
    );
}

// Test for querying delegator validators.
#[cfg(feature = "cosmwasm_1_4")]
#[test]
fn querying_delegator_validators_should_work() {
    // Prepare the delegator addresses.
    let delegator_1_addr = "delegator1".into_addr();
    let delegator_2_addr = "delegator2".into_addr();

    // Prepare validator addresses.
    let validator_1_addr = "valoper1".into_addr();
    let valoper1 = Validator::new(
        validator_1_addr.to_string(),
        Decimal::percent(10),
        Decimal::percent(90),
        Decimal::percent(1),
    );
    let validator_2_addr = "valoper2".into_addr();
    let valoper2 = Validator::new(
        validator_2_addr.to_string(),
        Decimal::percent(10),
        Decimal::percent(90),
        Decimal::percent(1),
    );

    // Prepare the blockchain.
    let block = mock_env().block;
    let mut app = AppBuilder::default().build(|router, api, storage| {
        // Set initial balances for all delegators
        router
            .bank
            .init_balance(
                storage,
                &delegator_1_addr,
                vec![coin(INITIAL_BALANCE, BONDED_DENOM)],
            )
            .unwrap();
        router
            .bank
            .init_balance(
                storage,
                &delegator_2_addr,
                vec![coin(INITIAL_BALANCE, BONDED_DENOM)],
            )
            .unwrap();
        // Set staking parameters.
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
        // Add all validators.
        router
            .staking
            .add_validator(api, storage, &block, valoper1)
            .unwrap();
        router
            .staking
            .add_validator(api, storage, &block, valoper2)
            .unwrap();
    });

    // Delegate tokens to validator 1 from delegator 1.
    app.execute(
        delegator_1_addr.clone(),
        StakingMsg::Delegate {
            validator: validator_1_addr.to_string(),
            amount: coin(DELEGATION_AMOUNT, BONDED_DENOM),
        }
        .into(),
    )
    .unwrap();

    // Delegate tokens to validator 2 from delegator 1.
    app.execute(
        delegator_1_addr.clone(),
        StakingMsg::Delegate {
            validator: validator_2_addr.to_string(),
            amount: coin(DELEGATION_AMOUNT, BONDED_DENOM),
        }
        .into(),
    )
    .unwrap();

    // Delegate tokens to validator 2 from delegator 2.
    app.execute(
        delegator_2_addr.clone(),
        StakingMsg::Delegate {
            validator: validator_2_addr.to_string(),
            amount: coin(DELEGATION_AMOUNT, BONDED_DENOM),
        }
        .into(),
    )
    .unwrap();

    // Query validators of delegator 1, should be two of them.
    let validators = app
        .wrap()
        .query_delegator_validators(delegator_1_addr)
        .unwrap();
    assert_eq!(2, validators.len());
    assert!(validators.contains(&validator_1_addr.to_string()));
    assert!(validators.contains(&validator_2_addr.to_string()));

    // Query validators of delegator 2, should be just one.
    let validators = app
        .wrap()
        .query_delegator_validators(delegator_2_addr)
        .unwrap();
    assert_eq!(1, validators.len());
    assert!(validators.contains(&validator_2_addr.to_string()));
}

#[cfg(feature = "cosmwasm_1_4")]
#[test]
fn querying_delegation_rewards_should_work() {
    // Prepare the delegator address.
    let delegator_addr = "delegator".into_addr();

    // Prepare the validator address.
    let validator_addr = "valoper".into_addr();

    // Prepare the validator.
    let valoper = Validator::new(
        validator_addr.to_string(),
        Decimal::percent(10),
        Decimal::percent(90),
        Decimal::percent(1),
    );

    // Prepare the blockchain.
    let block = mock_env().block;
    let mut app = AppBuilder::default().build(|router, api, storage| {
        // Set the initial balance of the delegator.
        router
            .bank
            .init_balance(
                storage,
                &delegator_addr,
                vec![coin(INITIAL_BALANCE, BONDED_DENOM)],
            )
            .unwrap();
        // Set staking parameters.
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

    // Delegate tokens to validator from delegator.
    app.execute(
        delegator_addr.clone(),
        StakingMsg::Delegate {
            validator: validator_addr.to_string(),
            amount: coin(DELEGATION_AMOUNT, BONDED_DENOM),
        }
        .into(),
    )
    .unwrap();

    // One year fast-forward.
    app.update_block(|block| {
        block.height += 1;
        block.time = block.time.plus_seconds(SECONDS_IN_YEAR);
    });

    // Query delegation rewards.
    let rewards = app
        .wrap()
        .query_delegation_rewards(delegator_addr, validator_addr)
        .unwrap();

    assert_eq!(1, rewards.len());
    let reward = &rewards[0];
    assert_eq!(
        Decimal256::from_atomics(Uint128::new(9), 0).unwrap(),
        reward.amount
    );
    assert_eq!(BONDED_DENOM, reward.denom);
}
