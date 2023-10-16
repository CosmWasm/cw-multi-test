use crate::app::CosmosRouter;
use crate::error::{anyhow, bail, AnyResult};
use crate::executor::AppResponse;
use crate::prefixed_storage::{prefixed, prefixed_read};
use crate::{BankSudo, Module};
use cosmwasm_std::{
    coin, ensure, ensure_eq, to_binary, Addr, AllDelegationsResponse, AllValidatorsResponse, Api,
    BankMsg, Binary, BlockInfo, BondedDenomResponse, Coin, CustomQuery, Decimal, Delegation,
    DelegationResponse, DistributionMsg, Empty, Event, FullDelegation, Querier, StakingMsg,
    StakingQuery, Storage, Timestamp, Uint128, Validator, ValidatorResponse,
};
use cw_storage_plus::{Deque, Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

// Contains some general staking parameters
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct StakingInfo {
    /// The denominator of the staking token
    pub bonded_denom: String,
    /// Time between unbonding and receiving tokens in seconds
    pub unbonding_time: u64,
    /// Interest rate per year (60 * 60 * 24 * 365 seconds)
    pub apr: Decimal,
}

impl Default for StakingInfo {
    fn default() -> Self {
        StakingInfo {
            bonded_denom: "TOKEN".to_string(),
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

/// Holds some operational data about a validator
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

// We need to expand on this, but we will need this to properly test out staking
#[derive(Clone, std::fmt::Debug, PartialEq, Eq, JsonSchema)]
pub enum StakingSudo {
    /// Slashes the given percentage of the validator's stake.
    /// For now, you cannot slash retrospectively in tests.
    Slash {
        validator: String,
        percentage: Decimal,
    },
    /// Causes the unbonding queue to be processed.
    /// This needs to be triggered manually, since there is no good place to do this right now.
    /// In cosmos-sdk, this is done in `EndBlock`, but we don't have that here.
    ProcessQueue {},
}

pub trait Staking: Module<ExecT = StakingMsg, QueryT = StakingQuery, SudoT = StakingSudo> {}

pub trait Distribution: Module<ExecT = DistributionMsg, QueryT = Empty, SudoT = Empty> {}

pub struct StakeKeeper {
    module_addr: Addr,
}

impl Default for StakeKeeper {
    fn default() -> Self {
        Self::new()
    }
}

impl StakeKeeper {
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
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        validator: Validator,
    ) -> AnyResult<()> {
        let mut storage = prefixed(storage, NAMESPACE_STAKING);

        let val_addr = api.addr_validate(&validator.address)?;
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
            amount: Uint128::new(1) * delegator_rewards, // multiplying by 1 to convert Decimal to Uint128
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
            / Decimal::from_ratio(60u128 * 60 * 24 * 365, 1u128);
        let commission = reward * validator_commission;

        reward - commission
    }

    /// Updates the staking reward for the given validator and their stakers
    /// It saves the validator info and it's stakers, so make sure not to overwrite that.
    /// Always call this to update rewards before changing anything that influences future rewards.
    fn update_rewards(
        api: &dyn Api,
        staking_storage: &mut dyn Storage,
        block: &BlockInfo,
        validator: &Addr,
    ) -> AnyResult<()> {
        let staking_info = Self::get_staking_info(staking_storage)?;

        let mut validator_info = VALIDATOR_INFO
            .may_load(staking_storage, validator)?
            // https://github.com/cosmos/cosmos-sdk/blob/3c5387048f75d7e78b40c5b8d2421fdb8f5d973a/x/staking/types/errors.go#L15
            .ok_or_else(|| anyhow!("validator does not exist"))?;

        let validator_obj = VALIDATOR_MAP.load(staking_storage, validator)?;

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
        VALIDATOR_INFO.save(staking_storage, validator, &validator_info)?;

        // update delegators
        if !new_rewards.is_zero() {
            let validator_addr = api.addr_validate(&validator_obj.address)?;
            // update all delegators
            for staker in validator_info.stakers.iter() {
                STAKES.update(
                    staking_storage,
                    (staker, &validator_addr),
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
    pub(crate) fn get_validator(
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

    pub(crate) fn get_stake(
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
                amount: Uint128::new(1) * shares.stake, // multiplying by 1 to convert Decimal to Uint128
            }
        }))
    }

    pub(crate) fn add_stake(
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

    pub(crate) fn remove_stake(
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
        validator_info.stake = validator_info.stake * remaining_percentage;

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
                ub.amount = ub.amount * remaining_percentage;
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
}

impl Staking for StakeKeeper {}

impl Module for StakeKeeper {
    type ExecT = StakingMsg;
    type QueryT = StakingQuery;
    type SudoT = StakingSudo;

    fn execute<ExecC, QueryC: CustomQuery>(
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
                let validator = api.addr_validate(&validator)?;

                // see https://github.com/cosmos/cosmos-sdk/blob/3c5387048f75d7e78b40c5b8d2421fdb8f5d973a/x/staking/types/msg.go#L202-L207
                if amount.amount.is_zero() {
                    bail!("invalid delegation amount");
                }

                // see https://github.com/cosmos/cosmos-sdk/blob/v0.46.1/x/staking/keeper/msg_server.go#L251-L256
                let events = vec![Event::new("delegate")
                    .add_attribute("validator", &validator)
                    .add_attribute("amount", format!("{}{}", amount.amount, amount.denom))
                    .add_attribute("new_shares", amount.amount.to_string())]; // TODO: calculate shares?
                self.add_stake(
                    api,
                    &mut staking_storage,
                    block,
                    &sender,
                    &validator,
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
                let validator = api.addr_validate(&validator)?;
                self.validate_denom(&staking_storage, &amount)?;

                // see https://github.com/cosmos/cosmos-sdk/blob/3c5387048f75d7e78b40c5b8d2421fdb8f5d973a/x/staking/types/msg.go#L292-L297
                if amount.amount.is_zero() {
                    bail!("invalid shares amount");
                }

                // see https://github.com/cosmos/cosmos-sdk/blob/v0.46.1/x/staking/keeper/msg_server.go#L378-L383
                let events = vec![Event::new("unbond")
                    .add_attribute("validator", &validator)
                    .add_attribute("amount", format!("{}{}", amount.amount, amount.denom))
                    .add_attribute("completion_time", "2022-09-27T14:00:00+00:00")]; // TODO: actual date?
                self.remove_stake(
                    api,
                    &mut staking_storage,
                    block,
                    &sender,
                    &validator,
                    amount.clone(),
                )?;
                // add tokens to unbonding queue
                let staking_info = Self::get_staking_info(&staking_storage)?;
                let mut unbonding_queue = UNBONDING_QUEUE
                    .may_load(&staking_storage)?
                    .unwrap_or_default();
                unbonding_queue.push_back(Unbonding {
                    delegator: sender.clone(),
                    validator,
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
                let src_validator = api.addr_validate(&src_validator)?;
                let dst_validator = api.addr_validate(&dst_validator)?;
                // see https://github.com/cosmos/cosmos-sdk/blob/v0.46.1/x/staking/keeper/msg_server.go#L316-L322
                let events = vec![Event::new("redelegate")
                    .add_attribute("source_validator", &src_validator)
                    .add_attribute("destination_validator", &dst_validator)
                    .add_attribute("amount", format!("{}{}", amount.amount, amount.denom))];

                self.remove_stake(
                    api,
                    &mut staking_storage,
                    block,
                    &sender,
                    &src_validator,
                    amount.clone(),
                )?;
                self.add_stake(
                    api,
                    &mut staking_storage,
                    block,
                    &sender,
                    &dst_validator,
                    amount,
                )?;

                Ok(AppResponse { events, data: None })
            }
            m => bail!("Unsupported staking message: {:?}", m),
        }
    }

    fn sudo<ExecC, QueryC: CustomQuery>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        msg: StakingSudo,
    ) -> AnyResult<AppResponse> {
        match msg {
            StakingSudo::Slash {
                validator,
                percentage,
            } => {
                let mut staking_storage = prefixed(storage, NAMESPACE_STAKING);
                let validator = api.addr_validate(&validator)?;
                self.validate_percentage(percentage)?;

                self.slash(api, &mut staking_storage, block, &validator, percentage)?;

                Ok(AppResponse::default())
            }
            StakingSudo::ProcessQueue {} => {
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
                                        .filter(|u| {
                                            u.delegator == delegator && u.validator == validator
                                        })
                                        .map(|u| u.amount)
                                        .sum::<Uint128>();
                                    stake
                                });
                            match delegation {
                                Some(delegation) if delegation.amount.is_zero() => {
                                    STAKES.remove(&mut staking_storage, (&delegator, &validator));
                                }
                                None => {
                                    STAKES.remove(&mut staking_storage, (&delegator, &validator))
                                }
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
                                        amount: vec![coin(
                                            amount.u128(),
                                            &staking_info.bonded_denom,
                                        )],
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
            StakingQuery::BondedDenom {} => Ok(to_binary(&BondedDenomResponse {
                denom: Self::get_staking_info(&staking_storage)?.bonded_denom,
            })?),
            StakingQuery::AllDelegations { delegator } => {
                let delegator = api.addr_validate(&delegator)?;
                let validators = self.get_validators(&staking_storage)?;

                let res: AnyResult<Vec<Delegation>> = validators
                    .into_iter()
                    .filter_map(|validator| {
                        let delegator = delegator.clone();
                        let amount = self
                            .get_stake(
                                &staking_storage,
                                &delegator,
                                &Addr::unchecked(&validator.address),
                            )
                            .transpose()?;

                        Some(amount.map(|amount| Delegation {
                            delegator,
                            validator: validator.address,
                            amount,
                        }))
                    })
                    .collect();

                Ok(to_binary(&AllDelegationsResponse { delegations: res? })?)
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
                let delegator = api.addr_validate(&delegator)?;

                let shares = STAKES
                    .may_load(&staking_storage, (&delegator, &validator_addr))?
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
                    (shares.stake * Uint128::new(1)).u128(),
                    staking_info.bonded_denom,
                );

                let full_delegation_response = if amount.amount.is_zero() {
                    // no delegation
                    DelegationResponse { delegation: None }
                } else {
                    DelegationResponse {
                        delegation: Some(FullDelegation {
                            delegator,
                            validator,
                            amount: amount.clone(),
                            can_redelegate: amount, // TODO: not implemented right now
                            accumulated_rewards: if reward.amount.is_zero() {
                                vec![]
                            } else {
                                vec![reward]
                            },
                        }),
                    }
                };

                let res = to_binary(&full_delegation_response)?;
                Ok(res)
            }
            StakingQuery::AllValidators {} => Ok(to_binary(&AllValidatorsResponse {
                validators: self.get_validators(&staking_storage)?,
            })?),
            StakingQuery::Validator { address } => Ok(to_binary(&ValidatorResponse {
                validator: self.get_validator(&staking_storage, &Addr::unchecked(address))?,
            })?),
            q => bail!("Unsupported staking sudo message: {:?}", q),
        }
    }
}

#[derive(Default)]
pub struct DistributionKeeper {}

impl DistributionKeeper {
    pub fn new() -> Self {
        DistributionKeeper {}
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
        let rewards = Uint128::new(1) * shares.rewards; // convert to Uint128

        // remove rewards from delegator
        shares.rewards = Decimal::zero();
        STAKES.save(&mut staking_storage, (delegator, validator), &shares)?;

        Ok(rewards)
    }

    pub fn get_withdraw_address(storage: &dyn Storage, delegator: &Addr) -> AnyResult<Addr> {
        Ok(match WITHDRAW_ADDRESS.may_load(storage, delegator)? {
            Some(a) => a,
            None => delegator.clone(),
        })
    }

    // https://docs.cosmos.network/main/modules/distribution#msgsetwithdrawaddress
    pub fn set_withdraw_address(
        storage: &mut dyn Storage,
        delegator: &Addr,
        withdraw_address: &Addr,
    ) -> AnyResult<()> {
        if delegator == withdraw_address {
            WITHDRAW_ADDRESS.remove(storage, delegator);
            Ok(())
        } else {
            // technically we should require that this address is not
            // the address of a module. TODO: how?
            WITHDRAW_ADDRESS
                .save(storage, delegator, withdraw_address)
                .map_err(|e| e.into())
        }
    }
}

impl Distribution for DistributionKeeper {}

impl Module for DistributionKeeper {
    type ExecT = DistributionMsg;
    type QueryT = Empty;
    type SudoT = Empty;

    fn execute<ExecC, QueryC: CustomQuery>(
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
                let validator_addr = api.addr_validate(&validator)?;

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
                    .add_attribute("validator", &validator)
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
}
