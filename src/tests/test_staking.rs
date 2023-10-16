use crate::app::MockRouter;
use crate::prefixed_storage::{prefixed, prefixed_read};
use crate::staking::NAMESPACE_STAKING;
use crate::{
    BankKeeper, DistributionKeeper, FailingModule, Module, Router, StakeKeeper, StakingInfo,
    StakingSudo, WasmKeeper,
};
use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};
use cosmwasm_std::{
    coin, from_slice, Addr, Api, BalanceResponse, BankQuery, BlockInfo, Decimal, DistributionMsg,
    Empty, GovMsg, IbcMsg, IbcQuery, Validator,
};

/// Year duration in seconds.
const YEAR: u64 = 60 * 60 * 24 * 365;

/// Type alias for default build `Router` to make its reference in typical scenario
type BasicRouter<ExecC = Empty, QueryC = Empty> = Router<
    BankKeeper,
    FailingModule<ExecC, QueryC, Empty>,
    WasmKeeper<ExecC, QueryC>,
    StakeKeeper,
    DistributionKeeper,
    FailingModule<IbcMsg, IbcQuery, Empty>,
    FailingModule<GovMsg, Empty, Empty>,
>;

fn mock_router() -> BasicRouter {
    Router {
        wasm: WasmKeeper::new(),
        bank: BankKeeper::new(),
        custom: FailingModule::new(),
        staking: StakeKeeper::new(),
        distribution: DistributionKeeper::new(),
        ibc: FailingModule::new(),
        gov: FailingModule::new(),
    }
}

fn setup_test_env(
    apr: Decimal,
    validator_commission: Decimal,
) -> (MockApi, MockStorage, BasicRouter, BlockInfo, Addr) {
    let api = MockApi::default();
    let router = mock_router();
    let mut store = MockStorage::new();
    let block = mock_env().block;

    let validator = api.addr_validate("testvaloper1").unwrap();

    router
        .staking
        .setup(
            &mut store,
            StakingInfo {
                bonded_denom: "TOKEN".to_string(),
                unbonding_time: 60,
                apr,
            },
        )
        .unwrap();

    // add validator
    let valoper1 = Validator {
        address: "testvaloper1".to_string(),
        commission: validator_commission,
        max_commission: Decimal::percent(100),
        max_change_rate: Decimal::percent(1),
    };
    router
        .staking
        .add_validator(&api, &mut store, &block, valoper1)
        .unwrap();

    (api, store, router, block, validator)
}

#[test]
fn add_get_validators() {
    let api = MockApi::default();
    let mut store = MockStorage::new();
    let stake = StakeKeeper::default();
    let block = mock_env().block;

    // add validator
    let valoper1 = Validator {
        address: "testvaloper1".to_string(),
        commission: Decimal::percent(10),
        max_commission: Decimal::percent(20),
        max_change_rate: Decimal::percent(1),
    };
    stake
        .add_validator(&api, &mut store, &block, valoper1.clone())
        .unwrap();

    // get it
    let staking_storage = prefixed_read(&store, NAMESPACE_STAKING);
    let val = stake
        .get_validator(
            &staking_storage,
            &api.addr_validate("testvaloper1").unwrap(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(val, valoper1);

    // try to add with same address
    let valoper1_fake = Validator {
        address: "testvaloper1".to_string(),
        commission: Decimal::percent(1),
        max_commission: Decimal::percent(10),
        max_change_rate: Decimal::percent(100),
    };
    stake
        .add_validator(&api, &mut store, &block, valoper1_fake)
        .unwrap_err();

    // should still be original value
    let staking_storage = prefixed_read(&store, NAMESPACE_STAKING);
    let val = stake
        .get_validator(
            &staking_storage,
            &api.addr_validate("testvaloper1").unwrap(),
        )
        .unwrap()
        .unwrap();
    assert_eq!(val, valoper1);
}

#[test]
fn validator_slashing() {
    let api = MockApi::default();
    let router = MockRouter::default();
    let mut store = MockStorage::new();
    let stake = StakeKeeper::new();
    let block = mock_env().block;

    let delegator = Addr::unchecked("delegator");
    let validator = api.addr_validate("testvaloper1").unwrap();

    // add validator
    let valoper1 = Validator {
        address: "testvaloper1".to_string(),
        commission: Decimal::percent(10),
        max_commission: Decimal::percent(20),
        max_change_rate: Decimal::percent(1),
    };
    stake
        .add_validator(&api, &mut store, &block, valoper1)
        .unwrap();

    // stake 100 tokens
    let mut staking_storage = prefixed(&mut store, NAMESPACE_STAKING);
    stake
        .add_stake(
            &api,
            &mut staking_storage,
            &block,
            &delegator,
            &validator,
            coin(100, "TOKEN"),
        )
        .unwrap();

    // slash 50%
    stake
        .sudo(
            &api,
            &mut store,
            &router,
            &block,
            StakingSudo::Slash {
                validator: "testvaloper1".to_string(),
                percentage: Decimal::percent(50),
            },
        )
        .unwrap();

    // check stake
    let staking_storage = prefixed(&mut store, NAMESPACE_STAKING);
    let stake_left = stake
        .get_stake(&staking_storage, &delegator, &validator)
        .unwrap();
    assert_eq!(
        stake_left.unwrap().amount.u128(),
        50,
        "should have slashed 50%"
    );

    // slash all
    stake
        .sudo(
            &api,
            &mut store,
            &router,
            &block,
            StakingSudo::Slash {
                validator: "testvaloper1".to_string(),
                percentage: Decimal::percent(100),
            },
        )
        .unwrap();

    // check stake
    let staking_storage = prefixed(&mut store, NAMESPACE_STAKING);
    let stake_left = stake
        .get_stake(&staking_storage, &delegator, &validator)
        .unwrap();
    assert_eq!(stake_left, None, "should have slashed whole stake");
}

#[test]
fn rewards_work_for_single_delegator() {
    let (api, mut store, router, mut block, validator) =
        setup_test_env(Decimal::percent(10), Decimal::percent(10));
    let stake = &router.staking;
    let distr = &router.distribution;
    let delegator = Addr::unchecked("delegator");

    let mut staking_storage = prefixed(&mut store, NAMESPACE_STAKING);
    // stake 200 tokens
    stake
        .add_stake(
            &api,
            &mut staking_storage,
            &block,
            &delegator,
            &validator,
            coin(200, "TOKEN"),
        )
        .unwrap();

    // wait 1/2 year
    block.time = block.time.plus_seconds(60 * 60 * 24 * 365 / 2);

    // should now have 200 * 10% / 2 - 10% commission = 9 tokens reward
    let rewards = stake
        .get_rewards(&store, &block, &delegator, &validator)
        .unwrap()
        .unwrap();
    assert_eq!(rewards.amount.u128(), 9, "should have 9 tokens reward");

    // withdraw rewards
    distr
        .execute(
            &api,
            &mut store,
            &router,
            &block,
            delegator.clone(),
            DistributionMsg::WithdrawDelegatorReward {
                validator: validator.to_string(),
            },
        )
        .unwrap();

    // should have no rewards left
    let rewards = stake
        .get_rewards(&store, &block, &delegator, &validator)
        .unwrap()
        .unwrap();
    assert_eq!(rewards.amount.u128(), 0);

    // wait another 1/2 year
    block.time = block.time.plus_seconds(60 * 60 * 24 * 365 / 2);
    // should now have 9 tokens again
    let rewards = stake
        .get_rewards(&store, &block, &delegator, &validator)
        .unwrap()
        .unwrap();
    assert_eq!(rewards.amount.u128(), 9);
}

#[test]
fn rewards_work_for_multiple_delegators() {
    let (api, mut store, router, mut block, validator) =
        setup_test_env(Decimal::percent(10), Decimal::percent(10));
    let stake = &router.staking;
    let distr = &router.distribution;
    let bank = &router.bank;
    let delegator1 = Addr::unchecked("delegator1");
    let delegator2 = Addr::unchecked("delegator2");

    let mut staking_storage = prefixed(&mut store, NAMESPACE_STAKING);

    // add 100 stake to delegator1 and 200 to delegator2
    stake
        .add_stake(
            &api,
            &mut staking_storage,
            &block,
            &delegator1,
            &validator,
            coin(100, "TOKEN"),
        )
        .unwrap();
    stake
        .add_stake(
            &api,
            &mut staking_storage,
            &block,
            &delegator2,
            &validator,
            coin(200, "TOKEN"),
        )
        .unwrap();

    // wait 1 year
    block.time = block.time.plus_seconds(60 * 60 * 24 * 365);

    // delegator1 should now have 100 * 10% - 10% commission = 9 tokens
    let rewards = stake
        .get_rewards(&store, &block, &delegator1, &validator)
        .unwrap()
        .unwrap();
    assert_eq!(rewards.amount.u128(), 9);

    // delegator2 should now have 200 * 10% - 10% commission = 18 tokens
    let rewards = stake
        .get_rewards(&store, &block, &delegator2, &validator)
        .unwrap()
        .unwrap();
    assert_eq!(rewards.amount.u128(), 18);

    // delegator1 stakes 100 more
    let mut staking_storage = prefixed(&mut store, NAMESPACE_STAKING);
    stake
        .add_stake(
            &api,
            &mut staking_storage,
            &block,
            &delegator1,
            &validator,
            coin(100, "TOKEN"),
        )
        .unwrap();

    // wait another year
    block.time = block.time.plus_seconds(60 * 60 * 24 * 365);

    // delegator1 should now have 9 + 200 * 10% - 10% commission = 27 tokens
    let rewards = stake
        .get_rewards(&store, &block, &delegator1, &validator)
        .unwrap()
        .unwrap();
    assert_eq!(rewards.amount.u128(), 27);

    // delegator2 should now have 18 + 200 * 10% - 10% commission = 36 tokens
    let rewards = stake
        .get_rewards(&store, &block, &delegator2, &validator)
        .unwrap()
        .unwrap();
    assert_eq!(rewards.amount.u128(), 36);

    // delegator2 unstakes 100 (has 100 left after that)
    let mut staking_storage = prefixed(&mut store, NAMESPACE_STAKING);
    stake
        .remove_stake(
            &api,
            &mut staking_storage,
            &block,
            &delegator2,
            &validator,
            coin(100, "TOKEN"),
        )
        .unwrap();

    // and delegator1 withdraws rewards
    distr
        .execute(
            &api,
            &mut store,
            &router,
            &block,
            delegator1.clone(),
            DistributionMsg::WithdrawDelegatorReward {
                validator: validator.to_string(),
            },
        )
        .unwrap();

    let balance: BalanceResponse = from_slice(
        &bank
            .query(
                &api,
                &store,
                &router.querier(&api, &store, &block),
                &block,
                BankQuery::Balance {
                    address: delegator1.to_string(),
                    denom: "TOKEN".to_string(),
                },
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        balance.amount.amount.u128(),
        27,
        "withdraw should change bank balance"
    );
    let rewards = stake
        .get_rewards(&store, &block, &delegator1, &validator)
        .unwrap()
        .unwrap();
    assert_eq!(
        rewards.amount.u128(),
        0,
        "withdraw should reduce rewards to 0"
    );

    // wait another year
    block.time = block.time.plus_seconds(60 * 60 * 24 * 365);

    // delegator1 should now have 0 + 200 * 10% - 10% commission = 18 tokens
    let rewards = stake
        .get_rewards(&store, &block, &delegator1, &validator)
        .unwrap()
        .unwrap();
    assert_eq!(rewards.amount.u128(), 18);

    // delegator2 should now have 36 + 100 * 10% - 10% commission = 45 tokens
    let rewards = stake
        .get_rewards(&store, &block, &delegator2, &validator)
        .unwrap()
        .unwrap();
    assert_eq!(rewards.amount.u128(), 45);
}

#[test]
fn rewards_should_fail_for_non_existing_validator() {
    let (api, mut store, router, mut block, validator) =
        setup_test_env(Decimal::percent(10), Decimal::percent(10));
    let stake = &router.staking;
    let delegator = Addr::unchecked("delegator");

    let mut staking_storage = prefixed(&mut store, NAMESPACE_STAKING);

    // stake 200 tokens
    stake
        .add_stake(
            &api,
            &mut staking_storage,
            &block,
            &delegator,
            &validator,
            coin(200, "TOKEN"),
        )
        .unwrap();

    // wait 1/2 year
    block.time = block.time.plus_seconds(YEAR / 2);

    // should fail because the address of non-existing validator was provided
    let invalid_validator = api.addr_validate("non-existing-validator").unwrap();
    assert_eq!(
        stake
            .get_rewards(&store, &block, &delegator, &invalid_validator)
            .unwrap_err()
            .to_string(),
        "validator non-existing-validator not found"
    );
}

#[test]
fn rewards_should_fail_for_invalid_stakes() {
    let (_, store, router, mut block, validator) =
        setup_test_env(Decimal::percent(10), Decimal::percent(10));
    let stake = &router.staking;
    let delegator = Addr::unchecked("delegator");

    // wait 1/2 year
    block.time = block.time.plus_seconds(YEAR / 2);

    // should fail because there are no stakes
    assert!(stake
        .get_rewards(&store, &block, &delegator, &validator)
        .unwrap()
        .is_none());
}

mod msg {
    use crate::error::AnyResult;
    use crate::AppResponse;
    use cosmwasm_std::{
        coins, from_slice, Addr, AllDelegationsResponse, AllValidatorsResponse,
        BondedDenomResponse, Decimal, Delegation, DelegationResponse, FullDelegation,
        QuerierWrapper, StakingMsg, StakingQuery, ValidatorResponse,
    };
    use serde::de::DeserializeOwned;

    use super::*;

    // shortens tests a bit
    struct TestEnv {
        api: MockApi,
        store: MockStorage,
        router: BasicRouter,
        block: BlockInfo,
    }

    impl TestEnv {
        fn wrap(tuple: (MockApi, MockStorage, BasicRouter, BlockInfo, Addr)) -> (Self, Addr) {
            (
                Self {
                    api: tuple.0,
                    store: tuple.1,
                    router: tuple.2,
                    block: tuple.3,
                },
                tuple.4,
            )
        }
    }

    fn execute_stake(env: &mut TestEnv, sender: Addr, msg: StakingMsg) -> AnyResult<AppResponse> {
        env.router.staking.execute(
            &env.api,
            &mut env.store,
            &env.router,
            &env.block,
            sender,
            msg,
        )
    }

    fn query_stake<T: DeserializeOwned>(env: &TestEnv, msg: StakingQuery) -> AnyResult<T> {
        Ok(from_slice(&env.router.staking.query(
            &env.api,
            &env.store,
            &env.router.querier(&env.api, &env.store, &env.block),
            &env.block,
            msg,
        )?)?)
    }

    fn execute_distr(
        env: &mut TestEnv,
        sender: Addr,
        msg: DistributionMsg,
    ) -> AnyResult<AppResponse> {
        env.router.distribution.execute(
            &env.api,
            &mut env.store,
            &env.router,
            &env.block,
            sender,
            msg,
        )
    }

    fn query_bank<T: DeserializeOwned>(env: &TestEnv, msg: BankQuery) -> AnyResult<T> {
        Ok(from_slice(&env.router.bank.query(
            &env.api,
            &env.store,
            &env.router.querier(&env.api, &env.store, &env.block),
            &env.block,
            msg,
        )?)?)
    }

    fn assert_balances(env: &TestEnv, balances: impl IntoIterator<Item = (Addr, u128)>) {
        for (addr, amount) in balances {
            let balance: BalanceResponse = query_bank(
                env,
                BankQuery::Balance {
                    address: addr.to_string(),
                    denom: "TOKEN".to_string(),
                },
            )
            .unwrap();
            assert_eq!(balance.amount.amount.u128(), amount);
        }
    }

    #[test]
    fn execute() {
        // test all execute msgs
        let (mut test_env, validator1) =
            TestEnv::wrap(setup_test_env(Decimal::percent(10), Decimal::percent(10)));

        let delegator1 = Addr::unchecked("delegator1");
        let reward_receiver = Addr::unchecked("rewardreceiver");

        // fund delegator1 account
        test_env
            .router
            .bank
            .init_balance(&mut test_env.store, &delegator1, vec![coin(1000, "TOKEN")])
            .unwrap();

        // add second validator
        let validator2 = Addr::unchecked("validator2");
        test_env
            .router
            .staking
            .add_validator(
                &test_env.api,
                &mut test_env.store,
                &test_env.block,
                Validator {
                    address: validator2.to_string(),
                    commission: Decimal::zero(),
                    max_commission: Decimal::percent(20),
                    max_change_rate: Decimal::percent(1),
                },
            )
            .unwrap();

        // delegate 100 tokens to validator1
        execute_stake(
            &mut test_env,
            delegator1.clone(),
            StakingMsg::Delegate {
                validator: validator1.to_string(),
                amount: coin(100, "TOKEN"),
            },
        )
        .unwrap();

        // should now have 100 tokens less
        assert_balances(&test_env, vec![(delegator1.clone(), 900)]);

        // wait a year
        test_env.block.time = test_env.block.time.plus_seconds(60 * 60 * 24 * 365);

        // change the withdrawal address
        execute_distr(
            &mut test_env,
            delegator1.clone(),
            DistributionMsg::SetWithdrawAddress {
                address: reward_receiver.to_string(),
            },
        )
        .unwrap();

        // withdraw rewards
        execute_distr(
            &mut test_env,
            delegator1.clone(),
            DistributionMsg::WithdrawDelegatorReward {
                validator: validator1.to_string(),
            },
        )
        .unwrap();

        // withdrawal address received rewards.
        assert_balances(
            &test_env,
            // one year, 10%apr, 10% commission, 100 tokens staked
            vec![(reward_receiver, 100 / 10 * 9 / 10)],
        );

        // redelegate to validator2
        execute_stake(
            &mut test_env,
            delegator1.clone(),
            StakingMsg::Redelegate {
                src_validator: validator1.to_string(),
                dst_validator: validator2.to_string(),
                amount: coin(100, "TOKEN"),
            },
        )
        .unwrap();

        // should have same amount as before (rewards receiver received rewards).
        assert_balances(&test_env, vec![(delegator1.clone(), 900)]);

        let delegations: AllDelegationsResponse = query_stake(
            &test_env,
            StakingQuery::AllDelegations {
                delegator: delegator1.to_string(),
            },
        )
        .unwrap();
        assert_eq!(
            delegations.delegations,
            [Delegation {
                delegator: delegator1.clone(),
                validator: validator2.to_string(),
                amount: coin(100, "TOKEN"),
            }]
        );

        // undelegate all tokens
        execute_stake(
            &mut test_env,
            delegator1.clone(),
            StakingMsg::Undelegate {
                validator: validator2.to_string(),
                amount: coin(100, "TOKEN"),
            },
        )
        .unwrap();

        // wait for unbonding period (60 seconds in default config)
        test_env.block.time = test_env.block.time.plus_seconds(60);

        // need to manually cause queue to get processed
        test_env
            .router
            .staking
            .sudo(
                &test_env.api,
                &mut test_env.store,
                &test_env.router,
                &test_env.block,
                StakingSudo::ProcessQueue {},
            )
            .unwrap();

        // check bank balance
        assert_balances(&test_env, vec![(delegator1.clone(), 1000)]);
    }

    #[test]
    fn can_set_withdraw_address() {
        let (mut test_env, validator) =
            TestEnv::wrap(setup_test_env(Decimal::percent(10), Decimal::percent(10)));

        let delegator = Addr::unchecked("delegator");
        let reward_receiver = Addr::unchecked("rewardreceiver");

        test_env
            .router
            .bank
            .init_balance(&mut test_env.store, &delegator, coins(100, "TOKEN"))
            .unwrap();

        // Stake 100 tokens to the validator.
        execute_stake(
            &mut test_env,
            delegator.clone(),
            StakingMsg::Delegate {
                validator: validator.to_string(),
                amount: coin(100, "TOKEN"),
            },
        )
        .unwrap();

        // Change rewards receiver.
        execute_distr(
            &mut test_env,
            delegator.clone(),
            DistributionMsg::SetWithdrawAddress {
                address: reward_receiver.to_string(),
            },
        )
        .unwrap();

        // A year passes.
        test_env.block.time = test_env.block.time.plus_seconds(60 * 60 * 24 * 365);

        // Withdraw rewards to reward receiver.
        execute_distr(
            &mut test_env,
            delegator.clone(),
            DistributionMsg::WithdrawDelegatorReward {
                validator: validator.to_string(),
            },
        )
        .unwrap();

        // Change reward receiver back to delegator.
        execute_distr(
            &mut test_env,
            delegator.clone(),
            DistributionMsg::SetWithdrawAddress {
                address: delegator.to_string(),
            },
        )
        .unwrap();

        // Another year passes.
        test_env.block.time = test_env.block.time.plus_seconds(60 * 60 * 24 * 365);

        // Withdraw rewards to delegator.
        execute_distr(
            &mut test_env,
            delegator.clone(),
            DistributionMsg::WithdrawDelegatorReward {
                validator: validator.to_string(),
            },
        )
        .unwrap();

        // one year, 10%apr, 10% commission, 100 tokens staked
        let rewards_yr = 100 / 10 * 9 / 10;

        assert_balances(
            &test_env,
            vec![(reward_receiver, rewards_yr), (delegator, rewards_yr)],
        );
    }

    #[test]
    fn cannot_steal() {
        let (mut test_env, validator1) =
            TestEnv::wrap(setup_test_env(Decimal::percent(10), Decimal::percent(10)));

        let delegator1 = Addr::unchecked("delegator1");

        // fund delegator1 account
        test_env
            .router
            .bank
            .init_balance(&mut test_env.store, &delegator1, vec![coin(100, "TOKEN")])
            .unwrap();

        // delegate 100 tokens to validator1
        execute_stake(
            &mut test_env,
            delegator1.clone(),
            StakingMsg::Delegate {
                validator: validator1.to_string(),
                amount: coin(100, "TOKEN"),
            },
        )
        .unwrap();

        // undelegate more tokens than we have
        let e = execute_stake(
            &mut test_env,
            delegator1.clone(),
            StakingMsg::Undelegate {
                validator: validator1.to_string(),
                amount: coin(200, "TOKEN"),
            },
        )
        .unwrap_err();

        assert_eq!(e.to_string(), "invalid shares amount");

        // add second validator
        let validator2 = Addr::unchecked("validator2");
        test_env
            .router
            .staking
            .add_validator(
                &test_env.api,
                &mut test_env.store,
                &test_env.block,
                Validator {
                    address: validator2.to_string(),
                    commission: Decimal::zero(),
                    max_commission: Decimal::percent(20),
                    max_change_rate: Decimal::percent(1),
                },
            )
            .unwrap();

        // redelegate more tokens than we have
        let e = execute_stake(
            &mut test_env,
            delegator1.clone(),
            StakingMsg::Redelegate {
                src_validator: validator1.to_string(),
                dst_validator: validator2.to_string(),
                amount: coin(200, "TOKEN"),
            },
        )
        .unwrap_err();
        assert_eq!(e.to_string(), "invalid shares amount");

        // undelegate from non-existing delegation
        let e = execute_stake(
            &mut test_env,
            delegator1.clone(),
            StakingMsg::Undelegate {
                validator: validator2.to_string(),
                amount: coin(100, "TOKEN"),
            },
        )
        .unwrap_err();
        assert_eq!(
            e.to_string(),
            "no delegation for (address, validator) tuple"
        );
    }

    #[test]
    fn denom_validation() {
        let (mut test_env, validator) =
            TestEnv::wrap(setup_test_env(Decimal::percent(10), Decimal::percent(10)));

        let delegator1 = Addr::unchecked("delegator1");

        // fund delegator1 account
        test_env
            .router
            .bank
            .init_balance(&mut test_env.store, &delegator1, vec![coin(100, "FAKE")])
            .unwrap();

        // try to delegate 100 to validator1
        let e = execute_stake(
            &mut test_env,
            delegator1.clone(),
            StakingMsg::Delegate {
                validator: validator.to_string(),
                amount: coin(100, "FAKE"),
            },
        )
        .unwrap_err();

        assert_eq!(
            e.to_string(),
            "cannot delegate coins of denominator FAKE, only of TOKEN",
        );
    }

    #[test]
    fn cannot_slash_nonexistent() {
        let (mut test_env, _) =
            TestEnv::wrap(setup_test_env(Decimal::percent(10), Decimal::percent(10)));

        let delegator1 = Addr::unchecked("delegator1");

        // fund delegator1 account
        test_env
            .router
            .bank
            .init_balance(&mut test_env.store, &delegator1, vec![coin(100, "FAKE")])
            .unwrap();

        // try to delegate 100 to validator1
        let e = test_env
            .router
            .staking
            .sudo(
                &test_env.api,
                &mut test_env.store,
                &test_env.router,
                &test_env.block,
                StakingSudo::Slash {
                    validator: "nonexistingvaloper".to_string(),
                    percentage: Decimal::percent(50),
                },
            )
            .unwrap_err();
        assert_eq!(e.to_string(), "validator does not exist");
    }

    #[test]
    fn non_existent_validator() {
        let (mut test_env, _) =
            TestEnv::wrap(setup_test_env(Decimal::percent(10), Decimal::percent(10)));

        let delegator = Addr::unchecked("delegator1");
        let validator = "testvaloper2";

        // init balances
        test_env
            .router
            .bank
            .init_balance(&mut test_env.store, &delegator, vec![coin(100, "TOKEN")])
            .unwrap();

        // try to delegate
        let err = execute_stake(
            &mut test_env,
            delegator.clone(),
            StakingMsg::Delegate {
                validator: validator.to_string(),
                amount: coin(100, "TOKEN"),
            },
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "validator does not exist");

        // try to undelegate
        let err = execute_stake(
            &mut test_env,
            delegator.clone(),
            StakingMsg::Undelegate {
                validator: validator.to_string(),
                amount: coin(100, "TOKEN"),
            },
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "validator does not exist");
    }

    #[test]
    fn zero_staking_forbidden() {
        let (mut test_env, validator) =
            TestEnv::wrap(setup_test_env(Decimal::percent(10), Decimal::percent(10)));

        let delegator = Addr::unchecked("delegator1");

        // delegate 0
        let err = execute_stake(
            &mut test_env,
            delegator.clone(),
            StakingMsg::Delegate {
                validator: validator.to_string(),
                amount: coin(0, "TOKEN"),
            },
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "invalid delegation amount");

        // undelegate 0
        let err = execute_stake(
            &mut test_env,
            delegator,
            StakingMsg::Undelegate {
                validator: validator.to_string(),
                amount: coin(0, "TOKEN"),
            },
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "invalid shares amount");
    }

    #[test]
    fn query_staking() {
        // run all staking queries
        let (mut test_env, validator1) =
            TestEnv::wrap(setup_test_env(Decimal::percent(10), Decimal::percent(10)));
        let delegator1 = Addr::unchecked("delegator1");
        let delegator2 = Addr::unchecked("delegator2");

        // init balances
        test_env
            .router
            .bank
            .init_balance(&mut test_env.store, &delegator1, vec![coin(260, "TOKEN")])
            .unwrap();
        test_env
            .router
            .bank
            .init_balance(&mut test_env.store, &delegator2, vec![coin(150, "TOKEN")])
            .unwrap();

        // add another validator
        let validator2 = test_env.api.addr_validate("testvaloper2").unwrap();
        let valoper2 = Validator {
            address: "testvaloper2".to_string(),
            commission: Decimal::percent(0),
            max_commission: Decimal::percent(1),
            max_change_rate: Decimal::percent(1),
        };
        test_env
            .router
            .staking
            .add_validator(
                &test_env.api,
                &mut test_env.store,
                &test_env.block,
                valoper2.clone(),
            )
            .unwrap();

        // query validators
        let valoper1: ValidatorResponse = query_stake(
            &test_env,
            StakingQuery::Validator {
                address: validator1.to_string(),
            },
        )
        .unwrap();
        let validators: AllValidatorsResponse =
            query_stake(&test_env, StakingQuery::AllValidators {}).unwrap();
        assert_eq!(
            validators.validators,
            [valoper1.validator.unwrap(), valoper2]
        );
        // query non-existent validator
        let response = query_stake::<ValidatorResponse>(
            &test_env,
            StakingQuery::Validator {
                address: "notvaloper".to_string(),
            },
        )
        .unwrap();
        assert_eq!(response.validator, None);

        // query bonded denom
        let response: BondedDenomResponse =
            query_stake(&test_env, StakingQuery::BondedDenom {}).unwrap();
        assert_eq!(response.denom, "TOKEN");

        // delegate some tokens with delegator1 and delegator2
        execute_stake(
            &mut test_env,
            delegator1.clone(),
            StakingMsg::Delegate {
                validator: validator1.to_string(),
                amount: coin(100, "TOKEN"),
            },
        )
        .unwrap();
        execute_stake(
            &mut test_env,
            delegator1.clone(),
            StakingMsg::Delegate {
                validator: validator2.to_string(),
                amount: coin(160, "TOKEN"),
            },
        )
        .unwrap();
        execute_stake(
            &mut test_env,
            delegator2.clone(),
            StakingMsg::Delegate {
                validator: validator1.to_string(),
                amount: coin(150, "TOKEN"),
            },
        )
        .unwrap();
        // unstake some again
        execute_stake(
            &mut test_env,
            delegator1.clone(),
            StakingMsg::Undelegate {
                validator: validator1.to_string(),
                amount: coin(50, "TOKEN"),
            },
        )
        .unwrap();
        execute_stake(
            &mut test_env,
            delegator2.clone(),
            StakingMsg::Undelegate {
                validator: validator1.to_string(),
                amount: coin(50, "TOKEN"),
            },
        )
        .unwrap();

        // query all delegations
        let response1: AllDelegationsResponse = query_stake(
            &test_env,
            StakingQuery::AllDelegations {
                delegator: delegator1.to_string(),
            },
        )
        .unwrap();
        assert_eq!(
            response1.delegations,
            vec![
                Delegation {
                    delegator: delegator1.clone(),
                    validator: validator1.to_string(),
                    amount: coin(50, "TOKEN"),
                },
                Delegation {
                    delegator: delegator1.clone(),
                    validator: validator2.to_string(),
                    amount: coin(160, "TOKEN"),
                },
            ]
        );
        let response2: DelegationResponse = query_stake(
            &test_env,
            StakingQuery::Delegation {
                delegator: delegator2.to_string(),
                validator: validator1.to_string(),
            },
        )
        .unwrap();
        assert_eq!(
            response2.delegation.unwrap(),
            FullDelegation {
                delegator: delegator2.clone(),
                validator: validator1.to_string(),
                amount: coin(100, "TOKEN"),
                accumulated_rewards: vec![],
                can_redelegate: coin(100, "TOKEN"),
            },
        );
    }

    #[test]
    fn delegation_queries_unbonding() {
        // run all staking queries
        let (mut test_env, validator) =
            TestEnv::wrap(setup_test_env(Decimal::percent(10), Decimal::percent(10)));
        let delegator1 = Addr::unchecked("delegator1");
        let delegator2 = Addr::unchecked("delegator2");

        // init balances
        test_env
            .router
            .bank
            .init_balance(&mut test_env.store, &delegator1, vec![coin(100, "TOKEN")])
            .unwrap();
        test_env
            .router
            .bank
            .init_balance(&mut test_env.store, &delegator2, vec![coin(150, "TOKEN")])
            .unwrap();

        // delegate some tokens with delegator1 and delegator2
        execute_stake(
            &mut test_env,
            delegator1.clone(),
            StakingMsg::Delegate {
                validator: validator.to_string(),
                amount: coin(100, "TOKEN"),
            },
        )
        .unwrap();
        execute_stake(
            &mut test_env,
            delegator2.clone(),
            StakingMsg::Delegate {
                validator: validator.to_string(),
                amount: coin(150, "TOKEN"),
            },
        )
        .unwrap();
        // unstake some of delegator1's stake
        execute_stake(
            &mut test_env,
            delegator1.clone(),
            StakingMsg::Undelegate {
                validator: validator.to_string(),
                amount: coin(50, "TOKEN"),
            },
        )
        .unwrap();
        // unstake all of delegator2's stake
        execute_stake(
            &mut test_env,
            delegator2.clone(),
            StakingMsg::Undelegate {
                validator: validator.to_string(),
                amount: coin(150, "TOKEN"),
            },
        )
        .unwrap();

        // query all delegations
        let response1: AllDelegationsResponse = query_stake(
            &test_env,
            StakingQuery::AllDelegations {
                delegator: delegator1.to_string(),
            },
        )
        .unwrap();
        assert_eq!(
            response1.delegations,
            vec![Delegation {
                delegator: delegator1.clone(),
                validator: validator.to_string(),
                amount: coin(50, "TOKEN"),
            }]
        );
        let response2: DelegationResponse = query_stake(
            &test_env,
            StakingQuery::Delegation {
                delegator: delegator2.to_string(),
                validator: validator.to_string(),
            },
        )
        .unwrap();
        assert_eq!(response2.delegation, None);

        // unstake rest of delegator1's stake in two steps
        execute_stake(
            &mut test_env,
            delegator1.clone(),
            StakingMsg::Undelegate {
                validator: validator.to_string(),
                amount: coin(25, "TOKEN"),
            },
        )
        .unwrap();
        test_env.block.time = test_env.block.time.plus_seconds(10);
        execute_stake(
            &mut test_env,
            delegator1.clone(),
            StakingMsg::Undelegate {
                validator: validator.to_string(),
                amount: coin(25, "TOKEN"),
            },
        )
        .unwrap();

        // query all delegations again
        let response1: DelegationResponse = query_stake(
            &test_env,
            StakingQuery::Delegation {
                delegator: delegator1.to_string(),
                validator: validator.to_string(),
            },
        )
        .unwrap();
        let response2: AllDelegationsResponse = query_stake(
            &test_env,
            StakingQuery::AllDelegations {
                delegator: delegator1.to_string(),
            },
        )
        .unwrap();
        assert_eq!(
            response1.delegation, None,
            "delegator1 should have no delegations left"
        );
        assert_eq!(response2.delegations, vec![]);
    }

    #[test]
    fn partial_unbonding_reduces_stake() {
        let (mut test_env, validator) =
            TestEnv::wrap(setup_test_env(Decimal::percent(10), Decimal::percent(10)));
        let delegator = Addr::unchecked("delegator1");

        // init balance
        test_env
            .router
            .bank
            .init_balance(&mut test_env.store, &delegator, vec![coin(100, "TOKEN")])
            .unwrap();

        // delegate all tokens
        execute_stake(
            &mut test_env,
            delegator.clone(),
            StakingMsg::Delegate {
                validator: validator.to_string(),
                amount: coin(100, "TOKEN"),
            },
        )
        .unwrap();
        // unstake in multiple steps
        execute_stake(
            &mut test_env,
            delegator.clone(),
            StakingMsg::Undelegate {
                validator: validator.to_string(),
                amount: coin(50, "TOKEN"),
            },
        )
        .unwrap();
        test_env.block.time = test_env.block.time.plus_seconds(10);
        execute_stake(
            &mut test_env,
            delegator.clone(),
            StakingMsg::Undelegate {
                validator: validator.to_string(),
                amount: coin(30, "TOKEN"),
            },
        )
        .unwrap();
        test_env.block.time = test_env.block.time.plus_seconds(10);
        execute_stake(
            &mut test_env,
            delegator.clone(),
            StakingMsg::Undelegate {
                validator: validator.to_string(),
                amount: coin(20, "TOKEN"),
            },
        )
        .unwrap();

        // wait for first unbonding to complete (but not the others) and process queue
        test_env.block.time = test_env.block.time.plus_seconds(40);
        test_env
            .router
            .staking
            .sudo(
                &test_env.api,
                &mut test_env.store,
                &test_env.router,
                &test_env.block,
                StakingSudo::ProcessQueue {},
            )
            .unwrap();

        // query delegations
        // we now have 0 stake, 50 unbonding and 50 completed unbonding
        let response1: DelegationResponse = query_stake(
            &test_env,
            StakingQuery::Delegation {
                delegator: delegator.to_string(),
                validator: validator.to_string(),
            },
        )
        .unwrap();
        let response2: AllDelegationsResponse = query_stake(
            &test_env,
            StakingQuery::AllDelegations {
                delegator: delegator.to_string(),
            },
        )
        .unwrap();
        assert_eq!(response1.delegation, None);
        assert_eq!(response2.delegations, vec![]);

        // wait for the rest to complete
        test_env.block.time = test_env.block.time.plus_seconds(20);
        test_env
            .router
            .staking
            .sudo(
                &test_env.api,
                &mut test_env.store,
                &test_env.router,
                &test_env.block,
                StakingSudo::ProcessQueue {},
            )
            .unwrap();

        // query delegations again
        let response1: DelegationResponse = query_stake(
            &test_env,
            StakingQuery::Delegation {
                delegator: delegator.to_string(),
                validator: validator.to_string(),
            },
        )
        .unwrap();
        let response2: AllDelegationsResponse = query_stake(
            &test_env,
            StakingQuery::AllDelegations {
                delegator: delegator.to_string(),
            },
        )
        .unwrap();
        assert_eq!(
            response1.delegation, None,
            "delegator should have nothing left"
        );
        assert!(response2.delegations.is_empty());
    }

    #[test]
    fn delegations_slashed() {
        // run all staking queries
        let (mut test_env, validator) =
            TestEnv::wrap(setup_test_env(Decimal::percent(10), Decimal::percent(10)));
        let delegator = Addr::unchecked("delegator");

        // init balance
        test_env
            .router
            .bank
            .init_balance(&mut test_env.store, &delegator, vec![coin(333, "TOKEN")])
            .unwrap();

        // delegate some tokens
        execute_stake(
            &mut test_env,
            delegator.clone(),
            StakingMsg::Delegate {
                validator: validator.to_string(),
                amount: coin(333, "TOKEN"),
            },
        )
        .unwrap();
        // unstake some
        execute_stake(
            &mut test_env,
            delegator.clone(),
            StakingMsg::Undelegate {
                validator: validator.to_string(),
                amount: coin(111, "TOKEN"),
            },
        )
        .unwrap();

        // slash validator
        test_env
            .router
            .staking
            .sudo(
                &test_env.api,
                &mut test_env.store,
                &test_env.router,
                &test_env.block,
                StakingSudo::Slash {
                    validator: "testvaloper1".to_string(),
                    percentage: Decimal::percent(50),
                },
            )
            .unwrap();

        // query all delegations
        let response1: AllDelegationsResponse = query_stake(
            &test_env,
            StakingQuery::AllDelegations {
                delegator: delegator.to_string(),
            },
        )
        .unwrap();
        assert_eq!(
            response1.delegations[0],
            Delegation {
                delegator: delegator.clone(),
                validator: validator.to_string(),
                amount: coin(111, "TOKEN"),
            }
        );

        // wait until unbonding is complete and check if amount was slashed
        test_env.block.time = test_env.block.time.plus_seconds(60);
        test_env
            .router
            .staking
            .sudo(
                &test_env.api,
                &mut test_env.store,
                &test_env.router,
                &test_env.block,
                StakingSudo::ProcessQueue {},
            )
            .unwrap();
        let balance = QuerierWrapper::<Empty>::new(&test_env.router.querier(
            &test_env.api,
            &test_env.store,
            &test_env.block,
        ))
        .query_balance(delegator, "TOKEN")
        .unwrap();
        assert_eq!(balance.amount.u128(), 55);
    }

    #[test]
    fn rewards_initial_wait() {
        let (mut test_env, validator) =
            TestEnv::wrap(setup_test_env(Decimal::percent(10), Decimal::zero()));
        let delegator = Addr::unchecked("delegator");

        // init balance
        test_env
            .router
            .bank
            .init_balance(&mut test_env.store, &delegator, vec![coin(100, "TOKEN")])
            .unwrap();

        // wait a year before staking
        test_env.block.time = test_env.block.time.plus_seconds(YEAR);

        // delegate some tokens
        execute_stake(
            &mut test_env,
            delegator.clone(),
            StakingMsg::Delegate {
                validator: validator.to_string(),
                amount: coin(100, "TOKEN"),
            },
        )
        .unwrap();

        // wait another year
        test_env.block.time = test_env.block.time.plus_seconds(YEAR);

        // query rewards
        let response: DelegationResponse = query_stake(
            &test_env,
            StakingQuery::Delegation {
                delegator: delegator.to_string(),
                validator: validator.to_string(),
            },
        )
        .unwrap();
        assert_eq!(
            response.delegation.unwrap().accumulated_rewards,
            vec![coin(10, "TOKEN")] // 10% of 100
        );
    }
}
