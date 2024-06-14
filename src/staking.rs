use crate::app::CosmosRouter;
use crate::error::{anyhow, bail, AnyResult};
use crate::executor::AppResponse;
use crate::prefixed_storage::{prefixed, prefixed_read};
use crate::{BankSudo, Module};
use cosmwasm_std::{
    coin, ensure, ensure_eq, to_json_binary, Addr, AllDelegationsResponse, AllValidatorsResponse,
    Api, BankMsg, Binary, BlockInfo, BondedDenomResponse, Coin, CustomMsg, CustomQuery, Decimal,
    Delegation, DelegationResponse, DistributionMsg, Empty, Event, FullDelegation, Querier,
    StakingMsg, StakingQuery, Storage, Timestamp, Uint128, Validator, ValidatorResponse,
};
use cw_storage_plus::{Deque, Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

/// Default denominator of the staking token.
const BONDED_DENOM: &str = "TOKEN";

/// One year expressed in seconds.
const YEAR: u64 = 60 * 60 * 24 * 365;

/// A structure containing some general staking parameters.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct StakingInfo {
    /// The denominator of the staking token.
    pub bonded_denom: String,
    /// Time between unbonding and receiving tokens back (in seconds).
    pub unbonding_time: u64,
    /// Annual percentage rate (interest rate and any additional fees associated with bonding).
    pub apr: Decimal,
}

impl Default for StakingInfo {
    /// Creates staking info with default settings.
    fn default() -> Self {
        StakingInfo {
            bonded_denom: BONDED_DENOM.to_string(),
            unbonding_time: 60,
            apr: Decimal::percent(10),
        }
    }
}

/// The number of stake and rewards of this validator the staker has. These can be fractional in case of slashing.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, JsonSchema)]
struct Shares {
    stake: Decimal,
    rewards: Decimal,
}

impl Shares {
    /// Calculates the share of validator rewards that should be given to this staker.
    pub fn share_of_rewards(&self, validator: &ValidatorInfo, rewards: Decimal) -> Decimal {
        if validator.stake.is_zero() {
            return Decimal::zero();
        }
        rewards * self.stake / validator.stake
    }
}

/// Holds some operational data about a validator.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct ValidatorInfo {
    /// The stakers that have staked with this validator.
    /// We need to track them for updating their rewards.
    stakers: BTreeSet<Addr>,
    /// The whole stake of all stakers
    stake: Uint128,
    /// The block time when this validator's rewards were last update. This is needed for rewards calculation.
    last_rewards_calculation: Timestamp,
}

impl ValidatorInfo {
    pub fn new(block_time: Timestamp) -> Self {
        Self {
            stakers: BTreeSet::new(),
            stake: Uint128::zero(),
            last_rewards_calculation: block_time,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct Unbonding {
    pub delegator: Addr,
    pub validator: Addr,
    pub amount: Uint128,
    pub payout_at: Timestamp,
}

const STAKING_INFO: Item<StakingInfo> = Item::new("staking_info");
/// (staker_addr, validator_addr) -> shares
const STAKES: Map<(&Addr, &Addr), Shares> = Map::new("stakes");
const VALIDATOR_MAP: Map<&Addr, Validator> = Map::new("validator_map");
/// Additional vec of validators, in case the `iterator` feature is disabled
const VALIDATORS: Deque<Validator> = Deque::new("validators");
/// Contains additional info for each validator
const VALIDATOR_INFO: Map<&Addr, ValidatorInfo> = Map::new("validator_info");
/// The queue of unbonding operations. This is needed because unbonding has a waiting time. See [`StakeKeeper`]
const UNBONDING_QUEUE: Item<VecDeque<Unbonding>> = Item::new("unbonding_queue");
/// (addr) -> addr. Maps addresses to the address they have delegated
/// to receive their staking rewards. A missing key => no delegation
/// has been set.
const WITHDRAW_ADDRESS: Map<&Addr, Addr> = Map::new("withdraw_address");

pub const NAMESPACE_STAKING: &[u8] = b"staking";
// https://github.com/cosmos/cosmos-sdk/blob/4f6f6c00021f4b5ee486bbb71ae2071a8ceb47c9/x/distribution/types/keys.go#L16
pub const NAMESPACE_DISTRIBUTION: &[u8] = b"distribution";

/// Staking privileged action definition.
///
/// We need to expand on this, but we will need this to properly test out staking
#[derive(Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum StakingSudo {
    /// Slashes the given percentage of the validator's stake.
    /// For now, you cannot slash retrospectively in tests.
    Slash {
        /// Validator's address.
        validator: String,
        /// Percentage of the validator's stake.
        percentage: Decimal,
    },
}

/// A trait defining a behavior of the stake keeper.
///
/// Manages staking operations, vital for testing contracts in proof-of-stake (PoS) blockchain environments.
/// This trait simulates staking behaviors, including delegation, validator operations, and reward mechanisms.
pub trait Staking: Module<ExecT = StakingMsg, QueryT = StakingQuery, SudoT = StakingSudo> {
    /// This is called from the end blocker (`update_block` / `set_block`) to process the
    /// staking queue. Needed because unbonding has a waiting time.
    /// If you're implementing a dummy staking module, this can be a no-op.
    fn process_queue<ExecC: CustomMsg, QueryC: CustomQuery>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
    ) -> AnyResult<AppResponse> {
        Ok(AppResponse::default())
    }
}

/// A trait defining a behavior of the distribution keeper.
pub trait Distribution: Module<ExecT = DistributionMsg, QueryT = Empty, SudoT = Empty> {}

/// A structure representing a default stake keeper.
pub struct StakeKeeper {
    /// Module address of a default stake keeper.
    module_addr: Addr,
}

impl Default for StakeKeeper {
    /// Creates a new stake keeper with default settings.
    fn default() -> Self {
        Self::new()
    }
}

impl StakeKeeper {
    /// Creates a new stake keeper with default module address.
    pub fn new() -> Self {
        StakeKeeper {
            // The address of the staking module. This holds all staked tokens.
            module_addr: Addr::unchecked("staking_module"),
        }
    }

    /// Provides some general parameters to the stake keeper
    pub fn setup(&self, storage: &mut dyn Storage, staking_info: StakingInfo) -> AnyResult<()> {
        let mut storage = prefixed(storage, NAMESPACE_STAKING);
        STAKING_INFO.save(&mut storage, &staking_info)?;
        Ok(())
    }

    /// Add a new validator available for staking
    pub fn add_validator(
        &self,
        _api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        validator: Validator,
    ) -> AnyResult<()> {
        let mut storage = prefixed(storage, NAMESPACE_STAKING);
        let val_addr = Addr::unchecked(validator.address.clone());
        if VALIDATOR_MAP.may_load(&storage, &val_addr)?.is_some() {
            bail!(
                "Cannot add validator {}, since a validator with that address already exists",
                val_addr
            );
        }
        VALIDATOR_MAP.save(&mut storage, &val_addr, &validator)?;
        VALIDATORS.push_back(&mut storage, &validator)?;
        VALIDATOR_INFO.save(&mut storage, &val_addr, &ValidatorInfo::new(block.time))?;
        Ok(())
    }

    fn get_staking_info(staking_storage: &dyn Storage) -> AnyResult<StakingInfo> {
        Ok(STAKING_INFO.may_load(staking_storage)?.unwrap_or_default())
    }

    /// Returns the rewards of the given delegator at the given validator
    pub fn get_rewards(
        &self,
        storage: &dyn Storage,
        block: &BlockInfo,
        delegator: &Addr,
        validator: &Addr,
    ) -> AnyResult<Option<Coin>> {
        let staking_storage = prefixed_read(storage, NAMESPACE_STAKING);

        let validator_obj = match self.get_validator(&staking_storage, validator)? {
            Some(validator) => validator,
            None => bail!("validator {} not found", validator),
        };
        // calculate rewards using fixed ratio
        let shares = match STAKES.load(&staking_storage, (delegator, validator)) {
            Ok(stakes) => stakes,
            Err(_) => {
                return Ok(None);
            }
        };
        let validator_info = VALIDATOR_INFO.load(&staking_storage, validator)?;

        Self::get_rewards_internal(
            &staking_storage,
            block,
            &shares,
            &validator_obj,
            &validator_info,
        )
        .map(Some)
    }

    fn get_rewards_internal(
        staking_storage: &dyn Storage,
        block: &BlockInfo,
        shares: &Shares,
        validator: &Validator,
        validator_info: &ValidatorInfo,
    ) -> AnyResult<Coin> {
        let staking_info = Self::get_staking_info(staking_storage)?;

        // calculate missing rewards without updating the validator to reduce rounding errors
        let new_validator_rewards = Self::calculate_rewards(
            block.time,
            validator_info.last_rewards_calculation,
            staking_info.apr,
            validator.commission,
            validator_info.stake,
        );

        // calculate the delegator's share of those
        let delegator_rewards =
            shares.rewards + shares.share_of_rewards(validator_info, new_validator_rewards);

        Ok(Coin {
            denom: staking_info.bonded_denom,
            amount: Uint128::new(1).mul_floor(delegator_rewards), // multiplying by 1 to convert Decimal to Uint128
        })
    }

    /// Calculates the rewards that are due since the last calculation.
    fn calculate_rewards(
        current_time: Timestamp,
        since: Timestamp,
        interest_rate: Decimal,
        validator_commission: Decimal,
        stake: Uint128,
    ) -> Decimal {
        // calculate time since last update (in seconds)
        let time_diff = current_time.minus_seconds(since.seconds()).seconds();

        // using decimal here to reduce rounding error when calling this function a lot
        let reward = Decimal::from_ratio(stake, 1u128)
            * interest_rate
            * Decimal::from_ratio(time_diff, 1u128)
            / Decimal::from_ratio(YEAR, 1u128);
        let commission = reward * validator_commission;

        reward - commission
    }

    /// Updates the staking reward for the given validator and their stakers
    /// It saves the validator info and stakers, so make sure not to overwrite that.
    /// Always call this to update rewards before changing anything that influences future rewards.
    fn update_rewards(
        _api: &dyn Api,
        staking_storage: &mut dyn Storage,
        block: &BlockInfo,
        validator_addr: &Addr,
    ) -> AnyResult<()> {
        let staking_info = Self::get_staking_info(staking_storage)?;

        let mut validator_info = VALIDATOR_INFO
            .may_load(staking_storage, validator_addr)?
            // https://github.com/cosmos/cosmos-sdk/blob/3c5387048f75d7e78b40c5b8d2421fdb8f5d973a/x/staking/types/errors.go#L15
            .ok_or_else(|| anyhow!("validator does not exist"))?;

        let validator_obj = VALIDATOR_MAP.load(staking_storage, validator_addr)?;

        if validator_info.last_rewards_calculation >= block.time {
            return Ok(());
        }

        let new_rewards = Self::calculate_rewards(
            block.time,
            validator_info.last_rewards_calculation,
            staking_info.apr,
            validator_obj.commission,
            validator_info.stake,
        );

        // update validator info
        validator_info.last_rewards_calculation = block.time;
        VALIDATOR_INFO.save(staking_storage, validator_addr, &validator_info)?;

        // update delegators
        if !new_rewards.is_zero() {
            let validator_obj_addr = Addr::unchecked(validator_obj.address);
            // update all delegators
            for staker in validator_info.stakers.iter() {
                STAKES.update(
                    staking_storage,
                    (staker, &validator_obj_addr),
                    |shares| -> AnyResult<_> {
                        let mut shares =
                            shares.expect("all stakers in validator_info should exist");
                        shares.rewards += shares.share_of_rewards(&validator_info, new_rewards);
                        Ok(shares)
                    },
                )?;
            }
        }
        Ok(())
    }

    /// Returns the single validator with the given address (or `None` if there is no such validator)
    fn get_validator(
        &self,
        staking_storage: &dyn Storage,
        address: &Addr,
    ) -> AnyResult<Option<Validator>> {
        Ok(VALIDATOR_MAP.may_load(staking_storage, address)?)
    }

    /// Returns all available validators
    fn get_validators(&self, staking_storage: &dyn Storage) -> AnyResult<Vec<Validator>> {
        let res: Result<_, _> = VALIDATORS.iter(staking_storage)?.collect();
        Ok(res?)
    }

    fn get_stake(
        &self,
        staking_storage: &dyn Storage,
        account: &Addr,
        validator: &Addr,
    ) -> AnyResult<Option<Coin>> {
        let shares = STAKES.may_load(staking_storage, (account, validator))?;
        let staking_info = Self::get_staking_info(staking_storage)?;

        Ok(shares.map(|shares| {
            Coin {
                denom: staking_info.bonded_denom,
                amount: Uint128::new(1).mul_floor(shares.stake), // multiplying by 1 to convert Decimal to Uint128
            }
        }))
    }

    fn add_stake(
        &self,
        api: &dyn Api,
        staking_storage: &mut dyn Storage,
        block: &BlockInfo,
        to_address: &Addr,
        validator: &Addr,
        amount: Coin,
    ) -> AnyResult<()> {
        self.validate_denom(staking_storage, &amount)?;
        self.update_stake(
            api,
            staking_storage,
            block,
            to_address,
            validator,
            amount.amount,
            false,
        )
    }

    fn remove_stake(
        &self,
        api: &dyn Api,
        staking_storage: &mut dyn Storage,
        block: &BlockInfo,
        from_address: &Addr,
        validator: &Addr,
        amount: Coin,
    ) -> AnyResult<()> {
        self.validate_denom(staking_storage, &amount)?;
        self.update_stake(
            api,
            staking_storage,
            block,
            from_address,
            validator,
            amount.amount,
            true,
        )
    }

    fn update_stake(
        &self,
        api: &dyn Api,
        staking_storage: &mut dyn Storage,
        block: &BlockInfo,
        delegator: &Addr,
        validator: &Addr,
        amount: impl Into<Uint128>,
        sub: bool,
    ) -> AnyResult<()> {
        let amount = amount.into();

        // update rewards for this validator
        Self::update_rewards(api, staking_storage, block, validator)?;

        // now, we can update the stake of the delegator and validator
        let mut validator_info = VALIDATOR_INFO
            .may_load(staking_storage, validator)?
            .unwrap_or_else(|| ValidatorInfo::new(block.time));
        let shares = STAKES.may_load(staking_storage, (delegator, validator))?;
        let mut shares = if sub {
            // see https://github.com/cosmos/cosmos-sdk/blob/3c5387048f75d7e78b40c5b8d2421fdb8f5d973a/x/staking/keeper/delegation.go#L1005-L1007
            // and https://github.com/cosmos/cosmos-sdk/blob/3c5387048f75d7e78b40c5b8d2421fdb8f5d973a/x/staking/types/errors.go#L31
            shares.ok_or_else(|| anyhow!("no delegation for (address, validator) tuple"))?
        } else {
            shares.unwrap_or_default()
        };

        let amount_dec = Decimal::from_ratio(amount, 1u128);
        if sub {
            // see https://github.com/cosmos/cosmos-sdk/blob/3c5387048f75d7e78b40c5b8d2421fdb8f5d973a/x/staking/keeper/delegation.go#L1019-L1022
            if amount_dec > shares.stake {
                bail!("invalid shares amount");
            }
            shares.stake -= amount_dec;
            validator_info.stake = validator_info.stake.checked_sub(amount)?;
        } else {
            shares.stake += amount_dec;
            validator_info.stake = validator_info.stake.checked_add(amount)?;
        }

        // save updated values
        if shares.stake.is_zero() {
            // no more stake, so remove
            STAKES.remove(staking_storage, (delegator, validator));
            validator_info.stakers.remove(delegator);
        } else {
            STAKES.save(staking_storage, (delegator, validator), &shares)?;
            validator_info.stakers.insert(delegator.clone());
        }
        // save updated validator info
        VALIDATOR_INFO.save(staking_storage, validator, &validator_info)?;

        Ok(())
    }

    fn slash(
        &self,
        api: &dyn Api,
        staking_storage: &mut dyn Storage,
        block: &BlockInfo,
        validator: &Addr,
        percentage: Decimal,
    ) -> AnyResult<()> {
        // calculate rewards before slashing
        Self::update_rewards(api, staking_storage, block, validator)?;

        // update stake of validator and stakers
        let mut validator_info = VALIDATOR_INFO
            .may_load(staking_storage, validator)?
            .unwrap();

        let remaining_percentage = Decimal::one() - percentage;
        validator_info.stake = validator_info.stake.mul_floor(remaining_percentage);

        // if the stake is completely gone, we clear all stakers and reinitialize the validator
        if validator_info.stake.is_zero() {
            // need to remove all stakes
            for delegator in validator_info.stakers.iter() {
                STAKES.remove(staking_storage, (delegator, validator));
            }
            validator_info.stakers.clear();
        } else {
            // otherwise we update all stakers
            for delegator in validator_info.stakers.iter() {
                STAKES.update(
                    staking_storage,
                    (delegator, validator),
                    |stake| -> AnyResult<_> {
                        let mut stake = stake.expect("all stakers in validator_info should exist");
                        stake.stake *= remaining_percentage;

                        Ok(stake)
                    },
                )?;
            }
        }
        // go through the queue to slash all pending unbondings
        let mut unbonding_queue = UNBONDING_QUEUE
            .may_load(staking_storage)?
            .unwrap_or_default();
        #[allow(clippy::op_ref)]
        unbonding_queue
            .iter_mut()
            .filter(|ub| &ub.validator == validator)
            .for_each(|ub| {
                ub.amount = ub.amount.mul_floor(remaining_percentage);
            });
        UNBONDING_QUEUE.save(staking_storage, &unbonding_queue)?;

        VALIDATOR_INFO.save(staking_storage, validator, &validator_info)?;
        Ok(())
    }

    // Asserts that the given coin has the proper denominator
    fn validate_denom(&self, staking_storage: &dyn Storage, amount: &Coin) -> AnyResult<()> {
        let staking_info = Self::get_staking_info(staking_storage)?;
        ensure_eq!(
            amount.denom,
            staking_info.bonded_denom,
            anyhow!(
                "cannot delegate coins of denominator {}, only of {}",
                amount.denom,
                staking_info.bonded_denom
            )
        );
        Ok(())
    }

    // Asserts that the given coin has the proper denominator
    fn validate_percentage(&self, percentage: Decimal) -> AnyResult<()> {
        ensure!(percentage <= Decimal::one(), anyhow!("expected percentage"));
        Ok(())
    }

    fn process_queue<ExecC: CustomMsg, QueryC: CustomQuery>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
    ) -> AnyResult<AppResponse> {
        let staking_storage = prefixed_read(storage, NAMESPACE_STAKING);
        let mut unbonding_queue = UNBONDING_QUEUE
            .may_load(&staking_storage)?
            .unwrap_or_default();
        loop {
            let mut staking_storage = prefixed(storage, NAMESPACE_STAKING);
            match unbonding_queue.front() {
                // assuming the queue is sorted by payout_at
                Some(Unbonding { payout_at, .. }) if payout_at <= &block.time => {
                    // remove from queue
                    let Unbonding {
                        delegator,
                        validator,
                        amount,
                        ..
                    } = unbonding_queue.pop_front().unwrap();

                    // remove staking entry if it is empty
                    let delegation = self
                        .get_stake(&staking_storage, &delegator, &validator)?
                        .map(|mut stake| {
                            // add unbonding amounts
                            stake.amount += unbonding_queue
                                .iter()
                                .filter(|u| u.delegator == delegator && u.validator == validator)
                                .map(|u| u.amount)
                                .sum::<Uint128>();
                            stake
                        });
                    match delegation {
                        Some(delegation) if delegation.amount.is_zero() => {
                            STAKES.remove(&mut staking_storage, (&delegator, &validator));
                        }
                        None => STAKES.remove(&mut staking_storage, (&delegator, &validator)),
                        _ => {}
                    }

                    let staking_info = Self::get_staking_info(&staking_storage)?;
                    if !amount.is_zero() {
                        router.execute(
                            api,
                            storage,
                            block,
                            self.module_addr.clone(),
                            BankMsg::Send {
                                to_address: delegator.into_string(),
                                amount: vec![coin(amount.u128(), &staking_info.bonded_denom)],
                            }
                            .into(),
                        )?;
                    }
                }
                _ => break,
            }
        }
        let mut staking_storage = prefixed(storage, NAMESPACE_STAKING);
        UNBONDING_QUEUE.save(&mut staking_storage, &unbonding_queue)?;
        Ok(AppResponse::default())
    }
}

impl Staking for StakeKeeper {
    fn process_queue<ExecC: CustomMsg, QueryC: CustomQuery>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
    ) -> AnyResult<AppResponse> {
        self.process_queue(api, storage, router, block)
    }
}

impl Module for StakeKeeper {
    type ExecT = StakingMsg;
    type QueryT = StakingQuery;
    type SudoT = StakingSudo;

    fn execute<ExecC: CustomMsg, QueryC: CustomQuery>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: StakingMsg,
    ) -> AnyResult<AppResponse> {
        let mut staking_storage = prefixed(storage, NAMESPACE_STAKING);
        match msg {
            StakingMsg::Delegate { validator, amount } => {
                let validator_addr = Addr::unchecked(validator);

                // see https://github.com/cosmos/cosmos-sdk/blob/3c5387048f75d7e78b40c5b8d2421fdb8f5d973a/x/staking/types/msg.go#L202-L207
                if amount.amount.is_zero() {
                    bail!("invalid delegation amount");
                }

                // see https://github.com/cosmos/cosmos-sdk/blob/v0.46.1/x/staking/keeper/msg_server.go#L251-L256
                let events = vec![Event::new("delegate")
                    .add_attribute("validator", &validator_addr)
                    .add_attribute("amount", format!("{}{}", amount.amount, amount.denom))
                    .add_attribute("new_shares", amount.amount.to_string())]; // TODO: calculate shares?
                self.add_stake(
                    api,
                    &mut staking_storage,
                    block,
                    &sender,
                    &validator_addr,
                    amount.clone(),
                )?;
                // move money from sender account to this module (note we can control sender here)
                router.execute(
                    api,
                    storage,
                    block,
                    sender,
                    BankMsg::Send {
                        to_address: self.module_addr.to_string(),
                        amount: vec![amount],
                    }
                    .into(),
                )?;
                Ok(AppResponse { events, data: None })
            }
            StakingMsg::Undelegate { validator, amount } => {
                let validator_addr = Addr::unchecked(validator);
                self.validate_denom(&staking_storage, &amount)?;

                // see https://github.com/cosmos/cosmos-sdk/blob/3c5387048f75d7e78b40c5b8d2421fdb8f5d973a/x/staking/types/msg.go#L292-L297
                if amount.amount.is_zero() {
                    bail!("invalid shares amount");
                }

                // see https://github.com/cosmos/cosmos-sdk/blob/v0.46.1/x/staking/keeper/msg_server.go#L378-L383
                let events = vec![Event::new("unbond")
                    .add_attribute("validator", &validator_addr)
                    .add_attribute("amount", format!("{}{}", amount.amount, amount.denom))
                    .add_attribute("completion_time", "2022-09-27T14:00:00+00:00")]; // TODO: actual date?
                self.remove_stake(
                    api,
                    &mut staking_storage,
                    block,
                    &sender,
                    &validator_addr,
                    amount.clone(),
                )?;
                // add tokens to unbonding queue
                let staking_info = Self::get_staking_info(&staking_storage)?;
                let mut unbonding_queue = UNBONDING_QUEUE
                    .may_load(&staking_storage)?
                    .unwrap_or_default();
                unbonding_queue.push_back(Unbonding {
                    delegator: sender.clone(),
                    validator: validator_addr,
                    amount: amount.amount,
                    payout_at: block.time.plus_seconds(staking_info.unbonding_time),
                });
                UNBONDING_QUEUE.save(&mut staking_storage, &unbonding_queue)?;
                Ok(AppResponse { events, data: None })
            }
            StakingMsg::Redelegate {
                src_validator,
                dst_validator,
                amount,
            } => {
                let src_validator_addr = Addr::unchecked(src_validator);
                let dst_validator_addr = Addr::unchecked(dst_validator);
                // see https://github.com/cosmos/cosmos-sdk/blob/v0.46.1/x/staking/keeper/msg_server.go#L316-L322
                let events = vec![Event::new("redelegate")
                    .add_attribute("source_validator", &src_validator_addr)
                    .add_attribute("destination_validator", &dst_validator_addr)
                    .add_attribute("amount", format!("{}{}", amount.amount, amount.denom))];

                self.remove_stake(
                    api,
                    &mut staking_storage,
                    block,
                    &sender,
                    &src_validator_addr,
                    amount.clone(),
                )?;
                self.add_stake(
                    api,
                    &mut staking_storage,
                    block,
                    &sender,
                    &dst_validator_addr,
                    amount,
                )?;

                Ok(AppResponse { events, data: None })
            }
            m => bail!("Unsupported staking message: {:?}", m),
        }
    }

    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        _querier: &dyn Querier,
        block: &BlockInfo,
        request: StakingQuery,
    ) -> AnyResult<Binary> {
        let staking_storage = prefixed_read(storage, NAMESPACE_STAKING);
        match request {
            StakingQuery::BondedDenom {} => Ok(to_json_binary(&BondedDenomResponse::new(
                Self::get_staking_info(&staking_storage)?.bonded_denom,
            ))?),
            StakingQuery::AllDelegations { delegator } => {
                let delegator_addr = api.addr_validate(&delegator)?;
                let validators = self.get_validators(&staking_storage)?;
                let res: AnyResult<Vec<Delegation>> = validators
                    .into_iter()
                    .filter_map(|validator| {
                        let delegator_addr_inner = delegator_addr.clone();
                        let amount = self
                            .get_stake(
                                &staking_storage,
                                &delegator_addr_inner,
                                &Addr::unchecked(&validator.address),
                            )
                            .transpose()?;
                        Some(amount.map(|amount| {
                            Delegation::new(delegator_addr_inner, validator.address, amount)
                        }))
                    })
                    .collect();
                Ok(to_json_binary(&AllDelegationsResponse::new(res?))?)
            }
            StakingQuery::Delegation {
                delegator,
                validator,
            } => {
                let validator_addr = Addr::unchecked(&validator);
                let validator_obj = match self.get_validator(&staking_storage, &validator_addr)? {
                    Some(validator) => validator,
                    None => bail!("non-existent validator {}", validator),
                };
                let delegator_addr = api.addr_validate(&delegator)?;

                let shares = STAKES
                    .may_load(&staking_storage, (&delegator_addr, &validator_addr))?
                    .unwrap_or_default();

                let validator_info = VALIDATOR_INFO.load(&staking_storage, &validator_addr)?;
                let reward = Self::get_rewards_internal(
                    &staking_storage,
                    block,
                    &shares,
                    &validator_obj,
                    &validator_info,
                )?;
                let staking_info = Self::get_staking_info(&staking_storage)?;

                let amount = coin(
                    Uint128::new(1).mul_floor(shares.stake).u128(),
                    staking_info.bonded_denom,
                );

                let full_delegation_response = if amount.amount.is_zero() {
                    // no delegation
                    DelegationResponse::new(None)
                } else {
                    DelegationResponse::new(Some(FullDelegation::new(
                        delegator_addr,
                        validator,
                        amount.clone(),
                        amount, // TODO: not implemented right now
                        if reward.amount.is_zero() {
                            vec![]
                        } else {
                            vec![reward]
                        },
                    )))
                };

                let res = to_json_binary(&full_delegation_response)?;
                Ok(res)
            }
            StakingQuery::AllValidators {} => Ok(to_json_binary(&AllValidatorsResponse::new(
                self.get_validators(&staking_storage)?,
            ))?),
            StakingQuery::Validator { address } => Ok(to_json_binary(&ValidatorResponse::new(
                self.get_validator(&staking_storage, &Addr::unchecked(address))?,
            ))?),
            q => bail!("Unsupported staking sudo message: {:?}", q),
        }
    }

    fn sudo<ExecC: CustomMsg, QueryC: CustomQuery>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        msg: StakingSudo,
    ) -> AnyResult<AppResponse> {
        match msg {
            StakingSudo::Slash {
                validator,
                percentage,
            } => {
                let mut staking_storage = prefixed(storage, NAMESPACE_STAKING);
                let validator_addr = Addr::unchecked(validator);
                self.validate_percentage(percentage)?;
                self.slash(
                    api,
                    &mut staking_storage,
                    block,
                    &validator_addr,
                    percentage,
                )?;
                Ok(AppResponse::default())
            }
        }
    }
}

/// A structure representing a default distribution keeper.
///
/// This module likely manages the distribution of rewards and fees within the blockchain network.
/// It could handle tasks like distributing block rewards to validators and delegators,
/// and managing community funding mechanisms.
#[derive(Default)]
pub struct DistributionKeeper {}

impl DistributionKeeper {
    /// Creates a new distribution keeper with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Removes all rewards from the given (delegator, validator) pair and returns the amount
    pub fn remove_rewards(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        delegator: &Addr,
        validator: &Addr,
    ) -> AnyResult<Uint128> {
        let mut staking_storage = prefixed(storage, NAMESPACE_STAKING);
        // update the validator and staker rewards
        StakeKeeper::update_rewards(api, &mut staking_storage, block, validator)?;

        // load updated rewards for delegator
        let mut shares = STAKES.load(&staking_storage, (delegator, validator))?;
        let rewards = Uint128::new(1).mul_floor(shares.rewards); // convert to Uint128

        // remove rewards from delegator
        shares.rewards = Decimal::zero();
        STAKES.save(&mut staking_storage, (delegator, validator), &shares)?;

        Ok(rewards)
    }

    /// Returns the withdrawal address for specified delegator.
    pub fn get_withdraw_address(storage: &dyn Storage, delegator: &Addr) -> AnyResult<Addr> {
        Ok(match WITHDRAW_ADDRESS.may_load(storage, delegator)? {
            Some(a) => a,
            None => delegator.clone(),
        })
    }

    /// Sets (changes) the [withdraw address] of the delegator.
    ///
    /// [withdraw address]: https://docs.cosmos.network/main/modules/distribution#msgsetwithdrawaddress
    pub fn set_withdraw_address(
        storage: &mut dyn Storage,
        delegator: &Addr,
        withdraw_addr: &Addr,
    ) -> AnyResult<()> {
        if delegator == withdraw_addr {
            WITHDRAW_ADDRESS.remove(storage, delegator);
            Ok(())
        } else {
            // technically we should require that this address is not
            // the address of a module. TODO: how?
            WITHDRAW_ADDRESS
                .save(storage, delegator, withdraw_addr)
                .map_err(|e| e.into())
        }
    }
}

impl Distribution for DistributionKeeper {}

impl Module for DistributionKeeper {
    type ExecT = DistributionMsg;
    type QueryT = Empty;
    type SudoT = Empty;

    fn execute<ExecC: CustomMsg, QueryC: CustomQuery>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: DistributionMsg,
    ) -> AnyResult<AppResponse> {
        match msg {
            DistributionMsg::WithdrawDelegatorReward { validator } => {
                let validator_addr = Addr::unchecked(validator);

                let rewards = self.remove_rewards(api, storage, block, &sender, &validator_addr)?;

                let staking_storage = prefixed_read(storage, NAMESPACE_STAKING);
                let distribution_storage = prefixed_read(storage, NAMESPACE_DISTRIBUTION);
                let staking_info = StakeKeeper::get_staking_info(&staking_storage)?;
                let receiver = Self::get_withdraw_address(&distribution_storage, &sender)?;
                // directly mint rewards to delegator
                router.sudo(
                    api,
                    storage,
                    block,
                    BankSudo::Mint {
                        to_address: receiver.into_string(),
                        amount: vec![Coin {
                            amount: rewards,
                            denom: staking_info.bonded_denom.clone(),
                        }],
                    }
                    .into(),
                )?;

                let events = vec![Event::new("withdraw_delegator_reward")
                    .add_attribute("validator", &validator_addr)
                    .add_attribute("sender", &sender)
                    .add_attribute(
                        "amount",
                        format!("{}{}", rewards, staking_info.bonded_denom),
                    )];
                Ok(AppResponse { events, data: None })
            }
            DistributionMsg::SetWithdrawAddress { address } => {
                let address = api.addr_validate(&address)?;
                // https://github.com/cosmos/cosmos-sdk/blob/4f6f6c00021f4b5ee486bbb71ae2071a8ceb47c9/x/distribution/keeper/msg_server.go#L38
                let storage = &mut prefixed(storage, NAMESPACE_DISTRIBUTION);
                Self::set_withdraw_address(storage, &sender, &address)?;
                Ok(AppResponse {
                    data: None,
                    // https://github.com/cosmos/cosmos-sdk/blob/4f6f6c00021f4b5ee486bbb71ae2071a8ceb47c9/x/distribution/keeper/keeper.go#L74
                    events: vec![Event::new("set_withdraw_address")
                        .add_attribute("withdraw_address", address)],
                })
            }
            m => bail!("Unsupported distribution message: {:?}", m),
        }
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _request: Empty,
    ) -> AnyResult<Binary> {
        bail!("Something went wrong - Distribution doesn't have query messages")
    }

    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _msg: Empty,
    ) -> AnyResult<AppResponse> {
        bail!("Something went wrong - Distribution doesn't have sudo messages")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        BankKeeper, FailingModule, GovFailingModule, IbcFailingModule, IntoBech32, Router,
        StargateFailing, WasmKeeper,
    };
    use cosmwasm_std::{
        from_json,
        testing::{mock_env, MockApi, MockStorage},
        BalanceResponse, BankQuery,
    };

    /// Type alias for default build of [Router], to make its reference in typical test scenario.
    type BasicRouter<ExecC = Empty, QueryC = Empty> = Router<
        BankKeeper,
        FailingModule<ExecC, QueryC, Empty>,
        WasmKeeper<ExecC, QueryC>,
        StakeKeeper,
        DistributionKeeper,
        IbcFailingModule,
        GovFailingModule,
        StargateFailing,
    >;

    fn mock_router() -> BasicRouter {
        Router {
            wasm: WasmKeeper::new(),
            bank: BankKeeper::new(),
            custom: FailingModule::new(),
            staking: StakeKeeper::new(),
            distribution: DistributionKeeper::new(),
            ibc: IbcFailingModule::new(),
            gov: GovFailingModule::new(),
            stargate: StargateFailing,
        }
    }

    fn valoper_addr(value: &str) -> Addr {
        value.into_bech32_with_prefix("cosmwasmvaloper")
    }

    type TestTuple = (MockApi, MockStorage, BasicRouter, BlockInfo, Addr);

    fn setup_test_env(apr: u64, validator_1: (u64, u64, u64)) -> TestTuple {
        let api = MockApi::default();
        let router = mock_router();
        let mut storage = MockStorage::new();
        let block = mock_env().block;

        let validator_addr = valoper_addr("validator1");

        router
            .staking
            .setup(
                &mut storage,
                StakingInfo {
                    bonded_denom: BONDED_DENOM.to_string(),
                    unbonding_time: 60,
                    apr: Decimal::percent(apr),
                },
            )
            .unwrap();

        // add validator
        let validator = Validator::new(
            validator_addr.to_string(),
            Decimal::percent(validator_1.0),
            Decimal::percent(validator_1.1),
            Decimal::percent(validator_1.2),
        );
        router
            .staking
            .add_validator(&api, &mut storage, &block, validator)
            .unwrap();

        (api, storage, router, block, validator_addr)
    }

    // shortens tests a bit
    struct TestEnv {
        api: MockApi,
        storage: MockStorage,
        router: BasicRouter,
        block: BlockInfo,
        validator_addr_1: Addr,
        validator_addr_2: Addr,
        delegator_addr_1: Addr,
        delegator_addr_2: Addr,
        user_addr_1: Addr,
    }

    impl TestEnv {
        fn wrap(tuple: TestTuple) -> Self {
            let api = tuple.0;
            let validator_addr_1 = tuple.4;
            let validator_addr_2 = valoper_addr("validator2");
            let delegator_addr_1 = api.addr_make("delegator1");
            let delegator_addr_2 = api.addr_make("delegator2");
            let user_addr_1 = api.addr_make("user1");
            Self {
                api,
                storage: tuple.1,
                router: tuple.2,
                block: tuple.3,
                validator_addr_1,
                validator_addr_2,
                delegator_addr_1,
                delegator_addr_2,
                user_addr_1,
            }
        }
    }

    #[test]
    fn add_get_validators() {
        let api = MockApi::default();
        let mut store = MockStorage::new();
        let stake = StakeKeeper::default();
        let block = mock_env().block;

        let validator_addr = valoper_addr("validator");

        // add validator
        let validator = Validator::new(
            validator_addr.to_string(),
            Decimal::percent(10),
            Decimal::percent(20),
            Decimal::percent(1),
        );
        stake
            .add_validator(&api, &mut store, &block, validator.clone())
            .unwrap();

        // get it
        let staking_storage = prefixed_read(&store, NAMESPACE_STAKING);
        let val = stake
            .get_validator(&staking_storage, &validator_addr)
            .unwrap()
            .unwrap();
        assert_eq!(val, validator);

        // try to add with same address
        let validator_fake = Validator::new(
            validator_addr.to_string(),
            Decimal::percent(1),
            Decimal::percent(10),
            Decimal::percent(100),
        );
        stake
            .add_validator(&api, &mut store, &block, validator_fake)
            .unwrap_err();

        // should still be original value
        let staking_storage = prefixed_read(&store, NAMESPACE_STAKING);
        let val = stake
            .get_validator(&staking_storage, &validator_addr)
            .unwrap()
            .unwrap();
        assert_eq!(val, validator);
    }

    #[test]
    fn validator_slashing() {
        let mut env = TestEnv::wrap(setup_test_env(10, (10, 100, 1)));

        let validator_addr = env.validator_addr_2.clone();
        let delegator_addr = env.delegator_addr_1.clone();

        // add validator
        let valoper1 = Validator::new(
            validator_addr.to_string(),
            Decimal::percent(10),
            Decimal::percent(20),
            Decimal::percent(1),
        );
        env.router
            .staking
            .add_validator(&env.api, &mut env.storage, &env.block, valoper1)
            .unwrap();

        // stake 100 tokens
        let mut staking_storage = prefixed(&mut env.storage, NAMESPACE_STAKING);
        env.router
            .staking
            .add_stake(
                &env.api,
                &mut staking_storage,
                &env.block,
                &delegator_addr,
                &validator_addr,
                coin(100, BONDED_DENOM),
            )
            .unwrap();

        // slash 50%
        env.router
            .staking
            .sudo(
                &env.api,
                &mut env.storage,
                &env.router,
                &env.block,
                StakingSudo::Slash {
                    validator: validator_addr.to_string(),
                    percentage: Decimal::percent(50),
                },
            )
            .unwrap();

        // check stake
        let staking_storage = prefixed(&mut env.storage, NAMESPACE_STAKING);
        let stake_left = env
            .router
            .staking
            .get_stake(&staking_storage, &delegator_addr, &validator_addr)
            .unwrap();
        assert_eq!(
            stake_left.unwrap().amount.u128(),
            50,
            "should have slashed 50%"
        );

        // slash all
        env.router
            .staking
            .sudo(
                &env.api,
                &mut env.storage,
                &env.router,
                &env.block,
                StakingSudo::Slash {
                    validator: validator_addr.to_string(),
                    percentage: Decimal::percent(100),
                },
            )
            .unwrap();

        // check stake
        let staking_storage = prefixed(&mut env.storage, NAMESPACE_STAKING);
        let stake_left = env
            .router
            .staking
            .get_stake(&staking_storage, &delegator_addr, &validator_addr)
            .unwrap();
        assert_eq!(stake_left, None, "should have slashed whole stake");
    }

    #[test]
    fn rewards_work_for_single_delegator() {
        let (api, mut store, router, mut block, validator_addr) = setup_test_env(10, (10, 100, 1));

        let delegator_addr = api.addr_make("delegator");

        let mut staking_storage = prefixed(&mut store, NAMESPACE_STAKING);
        // stake 200 tokens
        router
            .staking
            .add_stake(
                &api,
                &mut staking_storage,
                &block,
                &delegator_addr,
                &validator_addr,
                coin(200, BONDED_DENOM),
            )
            .unwrap();

        // wait 1/2 year
        block.time = block.time.plus_seconds(YEAR / 2);

        // should now have 200 * 10% / 2 - 10% commission = 9 tokens reward
        let rewards = router
            .staking
            .get_rewards(&store, &block, &delegator_addr, &validator_addr)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 9, "should have 9 tokens reward");

        // withdraw rewards
        router
            .distribution
            .execute(
                &api,
                &mut store,
                &router,
                &block,
                delegator_addr.clone(),
                DistributionMsg::WithdrawDelegatorReward {
                    validator: validator_addr.to_string(),
                },
            )
            .unwrap();

        // should have no rewards left
        let rewards = router
            .staking
            .get_rewards(&store, &block, &delegator_addr, &validator_addr)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 0);

        // wait another 1/2 year
        block.time = block.time.plus_seconds(YEAR / 2);
        // should now have 9 tokens again
        let rewards = router
            .staking
            .get_rewards(&store, &block, &delegator_addr, &validator_addr)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 9);
    }

    #[test]
    fn rewards_work_for_multiple_delegators() {
        let (api, mut store, router, mut block, validator_addr) = setup_test_env(10, (10, 100, 1));

        let delegator1 = api.addr_make("delegator1");
        let delegator2 = api.addr_make("delegator2");

        let mut staking_storage = prefixed(&mut store, NAMESPACE_STAKING);

        // add 100 stake to delegator1 and 200 to delegator2
        router
            .staking
            .add_stake(
                &api,
                &mut staking_storage,
                &block,
                &delegator1,
                &validator_addr,
                coin(100, BONDED_DENOM),
            )
            .unwrap();
        router
            .staking
            .add_stake(
                &api,
                &mut staking_storage,
                &block,
                &delegator2,
                &validator_addr,
                coin(200, BONDED_DENOM),
            )
            .unwrap();

        // wait 1 year
        block.time = block.time.plus_seconds(YEAR);

        // delegator1 should now have 100 * 10% - 10% commission = 9 tokens
        let rewards = router
            .staking
            .get_rewards(&store, &block, &delegator1, &validator_addr)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 9);

        // delegator2 should now have 200 * 10% - 10% commission = 18 tokens
        let rewards = router
            .staking
            .get_rewards(&store, &block, &delegator2, &validator_addr)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 18);

        // delegator1 stakes 100 more
        let mut staking_storage = prefixed(&mut store, NAMESPACE_STAKING);
        router
            .staking
            .add_stake(
                &api,
                &mut staking_storage,
                &block,
                &delegator1,
                &validator_addr,
                coin(100, BONDED_DENOM),
            )
            .unwrap();

        // wait another year
        block.time = block.time.plus_seconds(YEAR);

        // delegator1 should now have 9 + 200 * 10% - 10% commission = 27 tokens
        let rewards = router
            .staking
            .get_rewards(&store, &block, &delegator1, &validator_addr)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 27);

        // delegator2 should now have 18 + 200 * 10% - 10% commission = 36 tokens
        let rewards = router
            .staking
            .get_rewards(&store, &block, &delegator2, &validator_addr)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 36);

        // delegator2 unstakes 100 (has 100 left after that)
        let mut staking_storage = prefixed(&mut store, NAMESPACE_STAKING);
        router
            .staking
            .remove_stake(
                &api,
                &mut staking_storage,
                &block,
                &delegator2,
                &validator_addr,
                coin(100, BONDED_DENOM),
            )
            .unwrap();

        // and delegator1 withdraws rewards
        router
            .distribution
            .execute(
                &api,
                &mut store,
                &router,
                &block,
                delegator1.clone(),
                DistributionMsg::WithdrawDelegatorReward {
                    validator: validator_addr.to_string(),
                },
            )
            .unwrap();

        let balance: BalanceResponse = from_json(
            router
                .bank
                .query(
                    &api,
                    &store,
                    &router.querier(&api, &store, &block),
                    &block,
                    BankQuery::Balance {
                        address: delegator1.to_string(),
                        denom: BONDED_DENOM.to_string(),
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
        let rewards = router
            .staking
            .get_rewards(&store, &block, &delegator1, &validator_addr)
            .unwrap()
            .unwrap();
        assert_eq!(
            rewards.amount.u128(),
            0,
            "withdraw should reduce rewards to 0"
        );

        // wait another year
        block.time = block.time.plus_seconds(YEAR);

        // delegator1 should now have 0 + 200 * 10% - 10% commission = 18 tokens
        let rewards = router
            .staking
            .get_rewards(&store, &block, &delegator1, &validator_addr)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 18);

        // delegator2 should now have 36 + 100 * 10% - 10% commission = 45 tokens
        let rewards = router
            .staking
            .get_rewards(&store, &block, &delegator2, &validator_addr)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 45);
    }

    mod msg {
        use super::*;
        use cosmwasm_std::{coins, QuerierWrapper};
        use serde::de::DeserializeOwned;

        fn execute_stake(
            env: &mut TestEnv,
            sender: Addr,
            msg: StakingMsg,
        ) -> AnyResult<AppResponse> {
            env.router.staking.execute(
                &env.api,
                &mut env.storage,
                &env.router,
                &env.block,
                sender,
                msg,
            )
        }

        fn query_stake<T: DeserializeOwned>(env: &TestEnv, msg: StakingQuery) -> AnyResult<T> {
            Ok(from_json(env.router.staking.query(
                &env.api,
                &env.storage,
                &env.router.querier(&env.api, &env.storage, &env.block),
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
                &mut env.storage,
                &env.router,
                &env.block,
                sender,
                msg,
            )
        }

        fn query_bank<T: DeserializeOwned>(env: &TestEnv, msg: BankQuery) -> AnyResult<T> {
            Ok(from_json(env.router.bank.query(
                &env.api,
                &env.storage,
                &env.router.querier(&env.api, &env.storage, &env.block),
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
                        denom: BONDED_DENOM.to_string(),
                    },
                )
                .unwrap();
                assert_eq!(balance.amount.amount.u128(), amount);
            }
        }

        #[test]
        fn execute() {
            // test all execute msgs
            let mut env = TestEnv::wrap(setup_test_env(10, (10, 100, 1)));

            let validator_addr_1 = env.validator_addr_1.clone();
            let validator_addr_2 = env.validator_addr_2.clone();
            let delegator_addr_1 = env.delegator_addr_1.clone();
            let reward_receiver_addr = env.user_addr_1.clone();

            // fund delegator account
            env.router
                .bank
                .init_balance(
                    &mut env.storage,
                    &delegator_addr_1,
                    vec![coin(1000, BONDED_DENOM)],
                )
                .unwrap();

            // add second validator
            env.router
                .staking
                .add_validator(
                    &env.api,
                    &mut env.storage,
                    &env.block,
                    Validator::new(
                        validator_addr_2.to_string(),
                        Decimal::zero(),
                        Decimal::percent(20),
                        Decimal::percent(1),
                    ),
                )
                .unwrap();

            // delegate 100 tokens to validator 1
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Delegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(100, BONDED_DENOM),
                },
            )
            .unwrap();

            // should now have 100 tokens less
            assert_balances(&env, vec![(delegator_addr_1.clone(), 900)]);

            // wait a year
            env.block.time = env.block.time.plus_seconds(YEAR);

            // change the withdrawal address
            execute_distr(
                &mut env,
                delegator_addr_1.clone(),
                DistributionMsg::SetWithdrawAddress {
                    address: reward_receiver_addr.to_string(),
                },
            )
            .unwrap();

            // withdraw rewards
            execute_distr(
                &mut env,
                delegator_addr_1.clone(),
                DistributionMsg::WithdrawDelegatorReward {
                    validator: validator_addr_1.to_string(),
                },
            )
            .unwrap();

            // withdrawal address received rewards.
            assert_balances(
                &env,
                // one year, 10%apr, 10% commission, 100 tokens staked
                vec![(reward_receiver_addr, 100 / 10 * 9 / 10)],
            );

            // redelegate to validator2
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Redelegate {
                    src_validator: validator_addr_1.to_string(),
                    dst_validator: validator_addr_2.to_string(),
                    amount: coin(100, BONDED_DENOM),
                },
            )
            .unwrap();

            // should have same amount as before (rewards receiver received rewards).
            assert_balances(&env, vec![(delegator_addr_1.clone(), 900)]);

            let delegations: AllDelegationsResponse = query_stake(
                &env,
                StakingQuery::AllDelegations {
                    delegator: delegator_addr_1.to_string(),
                },
            )
            .unwrap();
            assert_eq!(
                delegations.delegations,
                [Delegation::new(
                    delegator_addr_1.clone(),
                    validator_addr_2.to_string(),
                    coin(100, BONDED_DENOM),
                )]
            );

            // undelegate all tokens
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Undelegate {
                    validator: validator_addr_2.to_string(),
                    amount: coin(100, BONDED_DENOM),
                },
            )
            .unwrap();

            // wait for unbonding period (60 seconds in default config)
            env.block.time = env.block.time.plus_seconds(60);

            // need to manually cause queue to get processed
            env.router
                .staking
                .process_queue(&env.api, &mut env.storage, &env.router, &env.block)
                .unwrap();

            // check bank balance
            assert_balances(&env, vec![(delegator_addr_1.clone(), 1000)]);
        }

        #[test]
        fn can_set_withdraw_address() {
            let mut env = TestEnv::wrap(setup_test_env(10, (10, 100, 1)));

            let validator_addr_1 = env.validator_addr_1.clone();
            let delegator_addr_1 = env.delegator_addr_1.clone();
            let reward_receiver_addr = env.user_addr_1.clone();

            // fund delegator's account
            env.router
                .bank
                .init_balance(&mut env.storage, &delegator_addr_1, coins(100, BONDED_DENOM))
                .unwrap();

            // stake (delegate) 100 tokens to the validator
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Delegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(100, BONDED_DENOM),
                },
            )
            .unwrap();

            // change rewards receiver
            execute_distr(
                &mut env,
                delegator_addr_1.clone(),
                DistributionMsg::SetWithdrawAddress {
                    address: reward_receiver_addr.to_string(),
                },
            )
            .unwrap();

            // let a year pass
            env.block.time = env.block.time.plus_seconds(YEAR);

            // withdraw rewards to reward receiver
            execute_distr(
                &mut env,
                delegator_addr_1.clone(),
                DistributionMsg::WithdrawDelegatorReward {
                    validator: validator_addr_1.to_string(),
                },
            )
            .unwrap();

            // change reward receiver back to delegator
            execute_distr(
                &mut env,
                delegator_addr_1.clone(),
                DistributionMsg::SetWithdrawAddress {
                    address: delegator_addr_1.to_string(),
                },
            )
            .unwrap();

            // let another year pass
            env.block.time = env.block.time.plus_seconds(YEAR);

            // withdraw rewards to delegator
            execute_distr(
                &mut env,
                delegator_addr_1.clone(),
                DistributionMsg::WithdrawDelegatorReward {
                    validator: validator_addr_1.to_string(),
                },
            )
            .unwrap();

            // one year, 10%apr, 10% commission, 100 tokens staked
            let rewards_yr = 100 / 10 * 9 / 10;

            assert_balances(
                &env,
                vec![
                    (reward_receiver_addr, rewards_yr),
                    (delegator_addr_1, rewards_yr),
                ],
            );
        }

        #[test]
        fn cannot_steal() {
            let mut env = TestEnv::wrap(setup_test_env(10, (10, 100, 1)));

            let validator_addr_1 = env.validator_addr_1.clone();
            let validator_addr_2 = env.validator_addr_2.clone();
            let delegator_addr_1 = env.delegator_addr_1.clone();

            // fund delegator's account
            env.router
                .bank
                .init_balance(
                    &mut env.storage,
                    &delegator_addr_1,
                    vec![coin(100, BONDED_DENOM)],
                )
                .unwrap();

            // stake (delegate) 100 tokens to validator 1
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Delegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(100, BONDED_DENOM),
                },
            )
            .unwrap();

            // undelegate more tokens than we previously delegated,
            // this operation should fail with appropriate error message
            let error = execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Undelegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(200, BONDED_DENOM),
                },
            )
            .unwrap_err();

            assert_eq!(error.to_string(), "invalid shares amount");

            // add second validator
            env.router
                .staking
                .add_validator(
                    &env.api,
                    &mut env.storage,
                    &env.block,
                    Validator::new(
                        validator_addr_2.to_string(),
                        Decimal::zero(),
                        Decimal::percent(20),
                        Decimal::percent(1),
                    ),
                )
                .unwrap();

            // redelegate more tokens than we have previously delegated
            // this operation should fail with appropriate error message
            let error = execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Redelegate {
                    src_validator: validator_addr_1.to_string(),
                    dst_validator: validator_addr_2.to_string(),
                    amount: coin(200, BONDED_DENOM),
                },
            )
            .unwrap_err();
            assert_eq!("invalid shares amount", error.to_string());

            // undelegate from non-existing delegation
            let error = execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Undelegate {
                    validator: validator_addr_2.to_string(),
                    amount: coin(100, BONDED_DENOM),
                },
            )
            .unwrap_err();
            assert_eq!(
                "no delegation for (address, validator) tuple",
                error.to_string(),
            );
        }

        #[test]
        fn denom_validation() {
            let mut test_env = TestEnv::wrap(setup_test_env(10, (10, 100, 1)));

            let validator_addr_1 = test_env.validator_addr_1.clone();
            let delegator_addr_1 = test_env.delegator_addr_1.clone();

            // fund delegator account
            test_env
                .router
                .bank
                .init_balance(
                    &mut test_env.storage,
                    &delegator_addr_1,
                    vec![coin(100, "FAKE")],
                )
                .unwrap();

            // try to stake (delegate) 100 fake tokens to validator
            let error = execute_stake(
                &mut test_env,
                delegator_addr_1.clone(),
                StakingMsg::Delegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(100, "FAKE"),
                },
            )
            .unwrap_err();

            assert_eq!(
                "cannot delegate coins of denominator FAKE, only of TOKEN",
                error.to_string(),
            );
        }

        #[test]
        fn cannot_slash_nonexistent() {
            let mut env = TestEnv::wrap(setup_test_env(10, (10, 100, 1)));

            let non_existing_validator = env.validator_addr_2.clone();
            let delegator_addr_1 = env.delegator_addr_1.clone();

            // fund delegator account
            env.router
                .bank
                .init_balance(&mut env.storage, &delegator_addr_1, vec![coin(100, "FAKE")])
                .unwrap();

            // try to delegate 100 tokens to non-existing validator
            // this operation should fail wit appropriate error message
            let error = env
                .router
                .staking
                .sudo(
                    &env.api,
                    &mut env.storage,
                    &env.router,
                    &env.block,
                    StakingSudo::Slash {
                        validator: non_existing_validator.to_string(),
                        percentage: Decimal::percent(50),
                    },
                )
                .unwrap_err();

            assert_eq!(error.to_string(), "validator does not exist");
        }

        #[test]
        fn non_existent_validator() {
            let mut env = TestEnv::wrap(setup_test_env(10, (10, 100, 1)));

            let non_existing_validator = env.validator_addr_2.clone(); // address of non-existing validator
            let delegator_addr_1 = env.delegator_addr_1.clone();

            // init balances for delegator
            env.router
                .bank
                .init_balance(
                    &mut env.storage,
                    &delegator_addr_1,
                    vec![coin(100, BONDED_DENOM)],
                )
                .unwrap();

            // try to delegate
            let err = execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Delegate {
                    validator: non_existing_validator.to_string(),
                    amount: coin(100, BONDED_DENOM),
                },
            )
            .unwrap_err();
            assert_eq!(err.to_string(), "validator does not exist");

            // try to undelegate
            let err = execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Undelegate {
                    validator: non_existing_validator.to_string(),
                    amount: coin(100, BONDED_DENOM),
                },
            )
            .unwrap_err();
            assert_eq!(err.to_string(), "validator does not exist");
        }

        #[test]
        fn zero_staking_forbidden() {
            let mut env = TestEnv::wrap(setup_test_env(10, (10, 100, 1)));

            let validator_addr_1 = env.validator_addr_1.clone();
            let delegator_addr_1 = env.delegator_addr_1.clone();

            // delegate 0 tokens
            let err = execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Delegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(0, BONDED_DENOM),
                },
            )
            .unwrap_err();
            assert_eq!(err.to_string(), "invalid delegation amount");

            // undelegate 0 tokens
            let err = execute_stake(
                &mut env,
                delegator_addr_1,
                StakingMsg::Undelegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(0, BONDED_DENOM),
                },
            )
            .unwrap_err();
            assert_eq!(err.to_string(), "invalid shares amount");
        }

        #[test]
        fn query_staking() {
            let mut env = TestEnv::wrap(setup_test_env(10, (10, 100, 1)));

            let validator_addr_1 = env.validator_addr_1.clone();
            let validator_addr_2 = env.validator_addr_2.clone();
            let delegator_addr_1 = env.delegator_addr_1.clone();
            let delegator_addr_2 = env.delegator_addr_2.clone();
            let non_validator_addr = env.user_addr_1.clone();

            // init balances for delegators
            env.router
                .bank
                .init_balance(
                    &mut env.storage,
                    &delegator_addr_1,
                    vec![coin(260, BONDED_DENOM)],
                )
                .unwrap();
            env.router
                .bank
                .init_balance(
                    &mut env.storage,
                    &delegator_addr_2,
                    vec![coin(150, BONDED_DENOM)],
                )
                .unwrap();

            // add another validator
            let valoper2 = Validator::new(
                validator_addr_2.to_string(),
                Decimal::percent(0),
                Decimal::percent(1),
                Decimal::percent(1),
            );
            env.router
                .staking
                .add_validator(&env.api, &mut env.storage, &env.block, valoper2.clone())
                .unwrap();

            // query validators
            let valoper1: ValidatorResponse = query_stake(
                &env,
                StakingQuery::Validator {
                    address: validator_addr_1.to_string(),
                },
            )
            .unwrap();
            let validators: AllValidatorsResponse =
                query_stake(&env, StakingQuery::AllValidators {}).unwrap();
            assert_eq!(
                validators.validators,
                [valoper1.validator.unwrap(), valoper2]
            );
            // query non-existent validator
            let response = query_stake::<ValidatorResponse>(
                &env,
                StakingQuery::Validator {
                    address: non_validator_addr.to_string(),
                },
            )
            .unwrap();
            assert_eq!(response.validator, None);

            // query bonded denom
            let response: BondedDenomResponse =
                query_stake(&env, StakingQuery::BondedDenom {}).unwrap();
            assert_eq!(response.denom, BONDED_DENOM);

            // delegate some tokens with delegator1 and delegator2
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Delegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(100, BONDED_DENOM),
                },
            )
            .unwrap();
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Delegate {
                    validator: validator_addr_2.to_string(),
                    amount: coin(160, BONDED_DENOM),
                },
            )
            .unwrap();
            execute_stake(
                &mut env,
                delegator_addr_2.clone(),
                StakingMsg::Delegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(150, BONDED_DENOM),
                },
            )
            .unwrap();
            // unstake some again
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Undelegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(50, BONDED_DENOM),
                },
            )
            .unwrap();
            execute_stake(
                &mut env,
                delegator_addr_2.clone(),
                StakingMsg::Undelegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(50, BONDED_DENOM),
                },
            )
            .unwrap();

            // query all delegations
            let response1: AllDelegationsResponse = query_stake(
                &env,
                StakingQuery::AllDelegations {
                    delegator: delegator_addr_1.to_string(),
                },
            )
            .unwrap();
            assert_eq!(
                response1.delegations,
                vec![
                    Delegation::new(
                        delegator_addr_1.clone(),
                        validator_addr_1.to_string(),
                        coin(50, BONDED_DENOM),
                    ),
                    Delegation::new(
                        delegator_addr_1.clone(),
                        validator_addr_2.to_string(),
                        coin(160, BONDED_DENOM),
                    ),
                ]
            );
            let response2: DelegationResponse = query_stake(
                &env,
                StakingQuery::Delegation {
                    delegator: delegator_addr_2.to_string(),
                    validator: validator_addr_1.to_string(),
                },
            )
            .unwrap();
            assert_eq!(
                response2.delegation.unwrap(),
                FullDelegation::new(
                    delegator_addr_2.clone(),
                    validator_addr_1.to_string(),
                    coin(100, BONDED_DENOM),
                    coin(100, BONDED_DENOM),
                    vec![],
                ),
            );
        }

        #[test]
        fn delegation_queries_unbonding() {
            let mut env = TestEnv::wrap(setup_test_env(10, (10, 100, 1)));

            let validator_addr_1 = env.validator_addr_1.clone();
            let delegator_addr_1 = env.delegator_addr_1.clone();
            let delegator_addr_2 = env.delegator_addr_2.clone();

            // init balances for delegators
            env.router
                .bank
                .init_balance(
                    &mut env.storage,
                    &delegator_addr_1,
                    vec![coin(100, BONDED_DENOM)],
                )
                .unwrap();
            env.router
                .bank
                .init_balance(
                    &mut env.storage,
                    &delegator_addr_2,
                    vec![coin(150, BONDED_DENOM)],
                )
                .unwrap();

            // delegate some tokens with delegator 1 and delegator 2
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Delegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(100, BONDED_DENOM),
                },
            )
            .unwrap();
            execute_stake(
                &mut env,
                delegator_addr_2.clone(),
                StakingMsg::Delegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(150, BONDED_DENOM),
                },
            )
            .unwrap();
            // unstake some of delegator1's stake
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Undelegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(50, BONDED_DENOM),
                },
            )
            .unwrap();
            // unstake all of delegator2's stake
            execute_stake(
                &mut env,
                delegator_addr_2.clone(),
                StakingMsg::Undelegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(150, BONDED_DENOM),
                },
            )
            .unwrap();

            // query all delegations
            let response1: AllDelegationsResponse = query_stake(
                &env,
                StakingQuery::AllDelegations {
                    delegator: delegator_addr_1.to_string(),
                },
            )
            .unwrap();
            assert_eq!(
                response1.delegations,
                vec![Delegation::new(
                    delegator_addr_1.clone(),
                    validator_addr_1.to_string(),
                    coin(50, BONDED_DENOM),
                )]
            );
            let response2: DelegationResponse = query_stake(
                &env,
                StakingQuery::Delegation {
                    delegator: delegator_addr_2.to_string(),
                    validator: validator_addr_1.to_string(),
                },
            )
            .unwrap();
            assert_eq!(response2.delegation, None);

            // unstake rest of delegator1's stake in two steps
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Undelegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(25, BONDED_DENOM),
                },
            )
            .unwrap();
            env.block.time = env.block.time.plus_seconds(10);
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Undelegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(25, BONDED_DENOM),
                },
            )
            .unwrap();

            // query all delegations again
            let response1: DelegationResponse = query_stake(
                &env,
                StakingQuery::Delegation {
                    delegator: delegator_addr_1.to_string(),
                    validator: validator_addr_1.to_string(),
                },
            )
            .unwrap();
            let response2: AllDelegationsResponse = query_stake(
                &env,
                StakingQuery::AllDelegations {
                    delegator: delegator_addr_1.to_string(),
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
            let mut env = TestEnv::wrap(setup_test_env(10, (10, 100, 1)));

            let validator_addr_1 = env.validator_addr_1.clone();
            let delegator_addr_1 = env.delegator_addr_1.clone();

            // init balance for delegator
            env.router
                .bank
                .init_balance(
                    &mut env.storage,
                    &delegator_addr_1,
                    vec![coin(100, BONDED_DENOM)],
                )
                .unwrap();

            // stake (delegate) all tokens to validator
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Delegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(100, BONDED_DENOM),
                },
            )
            .unwrap();

            // unstake in multiple steps
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Undelegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(50, BONDED_DENOM),
                },
            )
            .unwrap();
            env.block.time = env.block.time.plus_seconds(10);
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Undelegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(30, BONDED_DENOM),
                },
            )
            .unwrap();
            env.block.time = env.block.time.plus_seconds(10);
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Undelegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(20, BONDED_DENOM),
                },
            )
            .unwrap();

            // wait for first unbonding to complete (but not the others) and process queue
            env.block.time = env.block.time.plus_seconds(40);
            env.router
                .staking
                .process_queue(&env.api, &mut env.storage, &env.router, &env.block)
                .unwrap();

            // query delegations
            // we now have 0 stake, 50 unbonding and 50 completed unbonding
            let response1: DelegationResponse = query_stake(
                &env,
                StakingQuery::Delegation {
                    delegator: delegator_addr_1.to_string(),
                    validator: validator_addr_1.to_string(),
                },
            )
            .unwrap();
            let response2: AllDelegationsResponse = query_stake(
                &env,
                StakingQuery::AllDelegations {
                    delegator: delegator_addr_1.to_string(),
                },
            )
            .unwrap();
            assert_eq!(response1.delegation, None);
            assert_eq!(response2.delegations, vec![]);

            // wait for the rest to complete
            env.block.time = env.block.time.plus_seconds(20);
            env.router
                .staking
                .process_queue(&env.api, &mut env.storage, &env.router, &env.block)
                .unwrap();

            // query delegations again
            let response1: DelegationResponse = query_stake(
                &env,
                StakingQuery::Delegation {
                    delegator: delegator_addr_1.to_string(),
                    validator: validator_addr_1.to_string(),
                },
            )
            .unwrap();
            let response2: AllDelegationsResponse = query_stake(
                &env,
                StakingQuery::AllDelegations {
                    delegator: delegator_addr_1.to_string(),
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
            let mut env = TestEnv::wrap(setup_test_env(10, (10, 100, 1)));

            let validator_addr_1 = env.validator_addr_1.clone();
            let delegator_addr_1 = env.delegator_addr_1.clone();

            // init balance for delegator
            env.router
                .bank
                .init_balance(
                    &mut env.storage,
                    &delegator_addr_1,
                    vec![coin(333, BONDED_DENOM)],
                )
                .unwrap();

            // delegate some tokens to validator
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Delegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(333, BONDED_DENOM),
                },
            )
            .unwrap();

            // unstake (undelegate) some tokens
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Undelegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(111, BONDED_DENOM),
                },
            )
            .unwrap();

            // slash validator
            env.router
                .staking
                .sudo(
                    &env.api,
                    &mut env.storage,
                    &env.router,
                    &env.block,
                    StakingSudo::Slash {
                        validator: validator_addr_1.to_string(),
                        percentage: Decimal::percent(50),
                    },
                )
                .unwrap();

            // query all delegations
            let response1: AllDelegationsResponse = query_stake(
                &env,
                StakingQuery::AllDelegations {
                    delegator: delegator_addr_1.to_string(),
                },
            )
            .unwrap();

            assert_eq!(
                response1.delegations[0],
                Delegation::new(
                    delegator_addr_1.clone(),
                    validator_addr_1.to_string(),
                    coin(111, BONDED_DENOM)
                )
            );

            // wait until unbonding is completed and check if the proper amount was slashed
            env.block.time = env.block.time.plus_seconds(60);
            env.router
                .staking
                .process_queue(&env.api, &mut env.storage, &env.router, &env.block)
                .unwrap();
            let balance = QuerierWrapper::<Empty>::new(&env.router.querier(
                &env.api,
                &env.storage,
                &env.block,
            ))
            .query_balance(delegator_addr_1, BONDED_DENOM)
            .unwrap();

            assert_eq!(55, balance.amount.u128());
        }

        #[test]
        fn rewards_initial_wait() {
            let mut env = TestEnv::wrap(setup_test_env(10, (0, 100, 1)));

            let validator_addr_1 = env.validator_addr_1.clone();
            let delegator_addr_1 = env.delegator_addr_1.clone();

            // init balance
            env.router
                .bank
                .init_balance(
                    &mut env.storage,
                    &delegator_addr_1,
                    vec![coin(100, BONDED_DENOM)],
                )
                .unwrap();

            // wait a year before staking
            env.block.time = env.block.time.plus_seconds(YEAR);

            // delegate some tokens
            execute_stake(
                &mut env,
                delegator_addr_1.clone(),
                StakingMsg::Delegate {
                    validator: validator_addr_1.to_string(),
                    amount: coin(100, BONDED_DENOM),
                },
            )
            .unwrap();

            // wait another year
            env.block.time = env.block.time.plus_seconds(YEAR);

            // query rewards
            let response: DelegationResponse = query_stake(
                &env,
                StakingQuery::Delegation {
                    delegator: delegator_addr_1.to_string(),
                    validator: validator_addr_1.to_string(),
                },
            )
            .unwrap();
            assert_eq!(
                response.delegation.unwrap().accumulated_rewards,
                vec![coin(10, BONDED_DENOM)] // 10% of 100
            );
        }
    }
}
