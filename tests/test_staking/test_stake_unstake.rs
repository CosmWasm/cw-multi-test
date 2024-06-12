use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{coin, Decimal, StakingMsg, Validator};
use cw_multi_test::{AppBuilder, Executor, IntoBech32, StakingInfo};

#[test]
fn stake_unstake_should_work() {
    const BONDED_DENOM: &str = "stake"; // denominator of the staking token
    const UNBONDING_TIME: u64 = 60; // time between unbonding and receiving tokens back (in seconds)
    const DELEGATION_AMOUNT: u128 = 100; // amount of tokens to be (re)delegated
    const INITIAL_AMOUNT: u128 = 1000; // initial amount of tokens for delegator
    const FEWER_AMOUNT: u128 = INITIAL_AMOUNT - DELEGATION_AMOUNT; // amount of tokens after delegation

    let delegator_addr = "delegator".into_bech32();
    let validator_addr = "valoper".into_bech32();

    let valoper = Validator::new(
        validator_addr.to_string(),
        Decimal::percent(10),
        Decimal::percent(90),
        Decimal::percent(1),
    );

    // prepare the blockchain configuration
    let block = mock_env().block;
    let mut app = AppBuilder::default().build(|router, api, storage| {
        // set initial balance for the delegator
        router
            .bank
            .init_balance(
                storage,
                &delegator_addr,
                vec![coin(INITIAL_AMOUNT, BONDED_DENOM)],
            )
            .unwrap();
        // setup staking parameters
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
        // add a validator
        router
            .staking
            .add_validator(api, storage, &block, valoper)
            .unwrap();
    });

    // delegate tokens to validator
    app.execute(
        delegator_addr.clone(),
        StakingMsg::Delegate {
            validator: validator_addr.to_string(),
            amount: coin(DELEGATION_AMOUNT, BONDED_DENOM),
        }
        .into(),
    )
    .unwrap();

    // delegation works immediately, so delegator should have now fewer tokens
    let delegator_balance = app
        .wrap()
        .query_balance(delegator_addr.clone(), BONDED_DENOM)
        .unwrap();
    assert_eq!(FEWER_AMOUNT, delegator_balance.amount.u128());

    // validator should have now DELEGATION_AMOUNT of tokens assigned
    let delegation = app
        .wrap()
        .query_delegation(delegator_addr.clone(), validator_addr.clone())
        .unwrap()
        .unwrap();
    assert_eq!(DELEGATION_AMOUNT, delegation.amount.amount.u128());

    // now, undelegate all bonded tokens
    app.execute(
        delegator_addr.clone(),
        StakingMsg::Undelegate {
            validator: validator_addr.to_string(),
            amount: coin(DELEGATION_AMOUNT, BONDED_DENOM),
        }
        .into(),
    )
    .unwrap();

    // unbonding works with timeout, so tokens will be given back after unbonding time;
    // while we do not change the block size or time, delegator should still have fewer tokens
    let delegator_balance = app
        .wrap()
        .query_balance(delegator_addr.clone(), BONDED_DENOM)
        .unwrap();
    assert_eq!(FEWER_AMOUNT, delegator_balance.amount.u128());

    // now we update the block but with time that is shorter than unbonding time
    app.update_block(|block| {
        block.height += 1;
        block.time = block.time.plus_seconds(UNBONDING_TIME - 1);
    });

    // delegator should still have fewer tokens
    let delegator_balance = app
        .wrap()
        .query_balance(delegator_addr.clone(), BONDED_DENOM)
        .unwrap();
    assert_eq!(FEWER_AMOUNT, delegator_balance.amount.u128());

    // now we update the block so unbonding time is reached
    app.update_block(|block| {
        block.height += 1;
        block.time = block.time.plus_seconds(1);
    });

    // delegator should have back the initial amount of tokens
    let delegator_balance = app
        .wrap()
        .query_balance(delegator_addr.clone(), BONDED_DENOM)
        .unwrap();
    assert_eq!(INITIAL_AMOUNT, delegator_balance.amount.u128());

    // there should be no more delegations
    let delegation = app
        .wrap()
        .query_delegation(delegator_addr, validator_addr)
        .unwrap();
    assert_eq!(None, delegation);
}
