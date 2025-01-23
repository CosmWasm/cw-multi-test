use crate::app::CosmosRouter;
use crate::error::{bail, AnyResult};
use crate::executor::AppResponse;
use crate::ibc::types::{AppIbcBasicResponse, AppIbcReceiveResponse};
use crate::module::Module;
use crate::prefixed_storage::{prefixed, prefixed_read};
use cosmwasm_std::{
    coin, to_json_binary, Addr, AllBalanceResponse, Api, BalanceResponse, BankMsg, BankQuery,
    Binary, BlockInfo, Coin, DenomMetadata, Event, Querier, Storage,
};
#[cfg(feature = "cosmwasm_1_3")]
use cosmwasm_std::{AllDenomMetadataResponse, DenomMetadataResponse};
#[cfg(feature = "cosmwasm_1_1")]
use cosmwasm_std::{Order, StdResult, SupplyResponse, Uint128};
use cw_storage_plus::Map;
use cw_utils::NativeBalance;
use itertools::Itertools;
use schemars::JsonSchema;

use cosmwasm_std::{coins, from_json, IbcPacketAckMsg, IbcPacketReceiveMsg};
use cw20_ics20::ibc::Ics20Packet;

/// Collection of bank balances.
const BALANCES: Map<&Addr, NativeBalance> = Map::new("balances");

/// Collection of metadata for denomination.
const DENOM_METADATA: Map<String, DenomMetadata> = Map::new("metadata");

/// Default storage namespace for bank module.
const NAMESPACE_BANK: &[u8] = b"bank";
/// Default address for the locked IBC funds.
pub const IBC_LOCK_MODULE_ADDRESS: &str = "ibc_bank_lock_module";

/// A message representing privileged actions in bank module.
#[derive(Clone, Debug, PartialEq, Eq, JsonSchema)]
pub enum BankSudo {
    /// Minting privileged action.
    Mint {
        /// Destination address the tokens will be minted for.
        to_address: String,
        /// Amount of the minted tokens.
        amount: Vec<Coin>,
    },
}

/// This trait defines the interface for simulating banking operations.
///
/// In the test environment, it is essential for testing financial transactions,
/// like transfers and balance checks, within your smart contracts.
/// This trait implements all of these functionalities.
pub trait Bank: Module<ExecT = BankMsg, QueryT = BankQuery, SudoT = BankSudo> {}

/// A structure representing a default bank keeper.
///
/// Manages financial interactions in CosmWasm tests, such as simulating token transactions
/// and account balances. This is particularly important for contracts that deal with financial
/// operations in the Cosmos ecosystem.
#[derive(Default)]
pub struct BankKeeper {}

impl BankKeeper {
    /// Creates a new instance of a bank keeper with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Administration function for adjusting bank accounts in genesis.
    pub fn init_balance(
        &self,
        storage: &mut dyn Storage,
        account: &Addr,
        amount: Vec<Coin>,
    ) -> AnyResult<()> {
        let mut bank_storage = prefixed(storage, NAMESPACE_BANK);
        self.set_balance(&mut bank_storage, account, amount)
    }

    /// Administration function for adjusting bank accounts.
    fn set_balance(
        &self,
        bank_storage: &mut dyn Storage,
        account: &Addr,
        amount: Vec<Coin>,
    ) -> AnyResult<()> {
        let mut balance = NativeBalance(amount);
        balance.normalize();
        BALANCES
            .save(bank_storage, account, &balance)
            .map_err(Into::into)
    }

    /// Administration function for adjusting denomination metadata.
    pub fn set_denom_metadata(
        &self,
        bank_storage: &mut dyn Storage,
        denom: String,
        metadata: DenomMetadata,
    ) -> AnyResult<()> {
        DENOM_METADATA
            .save(bank_storage, denom, &metadata)
            .map_err(Into::into)
    }

    /// Returns balance for specified address.
    fn get_balance(&self, bank_storage: &dyn Storage, addr: &Addr) -> AnyResult<Vec<Coin>> {
        let val = BALANCES.may_load(bank_storage, addr)?;
        Ok(val.unwrap_or_default().into_vec())
    }

    #[cfg(feature = "cosmwasm_1_1")]
    fn get_supply(&self, bank_storage: &dyn Storage, denom: String) -> AnyResult<Coin> {
        let supply: Uint128 = BALANCES
            .range(bank_storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?
            .into_iter()
            .map(|a| a.1)
            .fold(Uint128::zero(), |accum, item| {
                let mut subtotal = Uint128::zero();
                for coin in item.into_vec() {
                    if coin.denom == denom {
                        subtotal += coin.amount;
                    }
                }
                accum + subtotal
            });
        Ok(coin(supply.into(), denom))
    }

    fn send(
        &self,
        bank_storage: &mut dyn Storage,
        from_address: Addr,
        to_address: Addr,
        amount: Vec<Coin>,
    ) -> AnyResult<()> {
        self.burn(bank_storage, from_address, amount.clone())?;
        self.mint(bank_storage, to_address, amount)
    }

    fn mint(
        &self,
        bank_storage: &mut dyn Storage,
        to_address: Addr,
        amount: Vec<Coin>,
    ) -> AnyResult<()> {
        let amount = self.normalize_amount(amount)?;
        let b = self.get_balance(bank_storage, &to_address)?;
        let b = NativeBalance(b) + NativeBalance(amount);
        self.set_balance(bank_storage, &to_address, b.into_vec())
    }

    fn burn(
        &self,
        bank_storage: &mut dyn Storage,
        from_address: Addr,
        amount: Vec<Coin>,
    ) -> AnyResult<()> {
        let amount = self.normalize_amount(amount)?;
        let a = self.get_balance(bank_storage, &from_address)?;
        let a = (NativeBalance(a) - amount)?;
        self.set_balance(bank_storage, &from_address, a.into_vec())
    }

    /// Filters out all `0` value coins and returns an error if the resulting vector is empty.
    fn normalize_amount(&self, amount: Vec<Coin>) -> AnyResult<Vec<Coin>> {
        let res: Vec<_> = amount.into_iter().filter(|x| !x.amount.is_zero()).collect();
        if res.is_empty() {
            bail!("Cannot transfer empty coins amount")
        } else {
            Ok(res)
        }
    }
}

fn coins_to_string(coins: &[Coin]) -> String {
    coins
        .iter()
        .map(|c| format!("{}{}", c.amount, c.denom))
        .join(",")
}

impl Bank for BankKeeper {}

impl Module for BankKeeper {
    type ExecT = BankMsg;
    type QueryT = BankQuery;
    type SudoT = BankSudo;

    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        sender: Addr,
        msg: BankMsg,
    ) -> AnyResult<AppResponse> {
        let mut bank_storage = prefixed(storage, NAMESPACE_BANK);
        match msg {
            BankMsg::Send { to_address, amount } => {
                // see https://github.com/cosmos/cosmos-sdk/blob/v0.42.7/x/bank/keeper/send.go#L142-L147
                let events = vec![Event::new("transfer")
                    .add_attribute("recipient", &to_address)
                    .add_attribute("sender", &sender)
                    .add_attribute("amount", coins_to_string(&amount))];
                self.send(
                    &mut bank_storage,
                    sender,
                    Addr::unchecked(to_address),
                    amount,
                )?;
                Ok(AppResponse {
                    events,
                    ..Default::default()
                })
            }
            BankMsg::Burn { amount } => {
                // burn doesn't seem to emit any events
                self.burn(&mut bank_storage, sender, amount)?;
                Ok(AppResponse::default())
            }
            other => unimplemented!("bank message: {other:?}"),
        }
    }

    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        request: BankQuery,
    ) -> AnyResult<Binary> {
        let bank_storage = prefixed_read(storage, NAMESPACE_BANK);
        match request {
            #[allow(deprecated)]
            BankQuery::AllBalances { address } => {
                let address = api.addr_validate(&address)?;
                let amount = self.get_balance(&bank_storage, &address)?;
                let res = AllBalanceResponse::new(amount);
                to_json_binary(&res).map_err(Into::into)
            }
            BankQuery::Balance { address, denom } => {
                let address = api.addr_validate(&address)?;
                let all_amounts = self.get_balance(&bank_storage, &address)?;
                let amount = all_amounts
                    .into_iter()
                    .find(|c| c.denom == denom)
                    .unwrap_or_else(|| coin(0, denom));
                let res = BalanceResponse::new(amount);
                to_json_binary(&res).map_err(Into::into)
            }
            #[cfg(feature = "cosmwasm_1_1")]
            BankQuery::Supply { denom } => {
                let amount = self.get_supply(&bank_storage, denom)?;
                let res = SupplyResponse::new(amount);
                to_json_binary(&res).map_err(Into::into)
            }
            #[cfg(feature = "cosmwasm_1_3")]
            BankQuery::DenomMetadata { denom } => {
                let meta = DENOM_METADATA.may_load(storage, denom)?.unwrap_or_default();
                let res = DenomMetadataResponse::new(meta);
                to_json_binary(&res).map_err(Into::into)
            }
            #[cfg(feature = "cosmwasm_1_3")]
            BankQuery::AllDenomMetadata { pagination: _ } => {
                let mut metadata = vec![];
                for key in DENOM_METADATA.keys(storage, None, None, Order::Ascending) {
                    metadata.push(DENOM_METADATA.may_load(storage, key?)?.unwrap_or_default());
                }
                let res = AllDenomMetadataResponse::new(metadata, None);
                to_json_binary(&res).map_err(Into::into)
            }
            other => unimplemented!("bank query: {other:?}"),
        }
    }

    fn sudo<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        msg: BankSudo,
    ) -> AnyResult<AppResponse> {
        let mut bank_storage = prefixed(storage, NAMESPACE_BANK);
        match msg {
            BankSudo::Mint { to_address, amount } => {
                let to_address = api.addr_validate(&to_address)?;
                self.mint(&mut bank_storage, to_address, amount)?;
                Ok(AppResponse::default())
            }
        }
    }

    fn ibc_packet_receive<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        request: IbcPacketReceiveMsg,
    ) -> AnyResult<AppIbcReceiveResponse> {
        // When receiving a packet, one simply needs to unpack the amount and send that to the the receiver
        let packet: Ics20Packet = from_json(&request.packet.data)?;

        let mut bank_storage = prefixed(storage, NAMESPACE_BANK);

        // If the denom is exactly a denom that was sent through this channel, we can mint it directly without denom changes
        // This can be verified by checking the ibc_module mock balance
        let balances =
            self.get_balance(&bank_storage, &Addr::unchecked(IBC_LOCK_MODULE_ADDRESS))?;
        let locked_amount = balances.iter().find(|b| b.denom == packet.denom);

        if let Some(locked_amount) = locked_amount {
            assert!(
                locked_amount.amount >= packet.amount,
                "The ibc locked amount is lower than the packet amount"
            );
            // We send tokens from the IBC_LOCK_MODULE
            self.send(
                &mut bank_storage,
                Addr::unchecked(IBC_LOCK_MODULE_ADDRESS),
                api.addr_validate(&packet.receiver)?,
                coins(packet.amount.u128(), packet.denom),
            )?;
        } else {
            // Else, we receive the denom with prefixes
            self.mint(
                &mut bank_storage,
                api.addr_validate(&packet.receiver)?,
                coins(
                    packet.amount.u128(),
                    wrap_ibc_denom(request.packet.dest.channel_id, packet.denom),
                ),
            )?;
        }

        // No acknowledgment needed
        Ok(AppIbcReceiveResponse::default())
    }

    fn ibc_packet_acknowledge<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _request: IbcPacketAckMsg,
    ) -> AnyResult<AppIbcBasicResponse> {
        // Acknowledgment can't fail, so no need for ack response parsing
        Ok(AppIbcBasicResponse::default())
    }

    fn ibc_packet_timeout<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        request: cosmwasm_std::IbcPacketTimeoutMsg,
    ) -> AnyResult<AppIbcBasicResponse> {
        // On timeout, we unpack the amount and sent that back to the receiverwe give the funds back to the sender of the packet

        // When receiving a packet, one simply needs to unpack the amount and send that to the the receiver
        let packet: Ics20Packet = from_json(request.packet.data)?;

        let mut bank_storage = prefixed(storage, NAMESPACE_BANK);

        // We verify the denom is exactly a denom that was sent through this channel
        // This can be verified by checking the ibc_module mock balance
        let balances =
            self.get_balance(&bank_storage, &Addr::unchecked(IBC_LOCK_MODULE_ADDRESS))?;
        let locked_amount = balances.iter().find(|b| b.denom == packet.denom);

        if let Some(locked_amount) = locked_amount {
            assert!(
                locked_amount.amount >= packet.amount,
                "The ibc locked amount is lower than the packet amount"
            );
            // We send tokens from the IBC_LOCK_MODULE
            self.send(
                &mut bank_storage,
                Addr::unchecked(IBC_LOCK_MODULE_ADDRESS),
                api.addr_validate(&packet.sender)?,
                coins(packet.amount.u128(), packet.denom),
            )?;
        } else {
            bail!("Funds refund after a timeout, can't timeout a transfer that was not initiated")
        }

        Ok(AppIbcBasicResponse::default())
    }
}

pub fn wrap_ibc_denom(channel_id: String, denom: String) -> String {
    format!("ibc/{}/{}", channel_id, denom)
}

pub fn optional_unwrap_ibc_denom(denom: String, expected_channel_id: String) -> String {
    let split: Vec<_> = denom.splitn(3, '/').collect();
    if split.len() != 3 {
        return denom;
    }

    if split[0] != "ibc" {
        return denom;
    }

    if split[1] != expected_channel_id {
        return denom;
    }

    split[2].to_string()
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::app::MockRouter;
    use cosmwasm_std::testing::{mock_env, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{coins, from_json, Empty, StdError};

    fn query_balance(
        bank: &BankKeeper,
        api: &dyn Api,
        store: &dyn Storage,
        rcpt: &Addr,
    ) -> Vec<Coin> {
        #[allow(deprecated)]
        let req = BankQuery::AllBalances {
            address: rcpt.clone().into(),
        };
        let block = mock_env().block;
        let querier: MockQuerier<Empty> = MockQuerier::new(&[]);

        let raw = bank.query(api, store, &querier, &block, req).unwrap();
        let res: AllBalanceResponse = from_json(raw).unwrap();
        res.amount
    }

    #[test]
    #[cfg(feature = "cosmwasm_1_1")]
    fn get_set_balance() {
        let api = MockApi::default();
        let mut store = MockStorage::new();
        let block = mock_env().block;
        let querier: MockQuerier<Empty> = MockQuerier::new(&[]);
        let router = MockRouter::default();

        let owner = api.addr_make("owner");
        let rcpt = api.addr_make("receiver");
        let init_funds = vec![coin(100, "eth"), coin(20, "btc")];
        let norm = vec![coin(20, "btc"), coin(100, "eth")];

        // set money
        let bank = BankKeeper::new();
        bank.init_balance(&mut store, &owner, init_funds).unwrap();
        let bank_storage = prefixed_read(&store, NAMESPACE_BANK);

        // get balance work
        let rich = bank.get_balance(&bank_storage, &owner).unwrap();
        assert_eq!(rich, norm);
        let poor = bank.get_balance(&bank_storage, &rcpt).unwrap();
        assert_eq!(poor, vec![]);

        // proper queries work
        #[allow(deprecated)]
        let req = BankQuery::AllBalances {
            address: owner.clone().into(),
        };
        let raw = bank.query(&api, &store, &querier, &block, req).unwrap();
        let res: AllBalanceResponse = from_json(raw).unwrap();
        assert_eq!(res.amount, norm);

        #[allow(deprecated)]
        let req = BankQuery::AllBalances {
            address: rcpt.clone().into(),
        };
        let raw = bank.query(&api, &store, &querier, &block, req).unwrap();
        let res: AllBalanceResponse = from_json(raw).unwrap();
        assert_eq!(res.amount, vec![]);

        let req = BankQuery::Balance {
            address: owner.clone().into(),
            denom: "eth".into(),
        };
        let raw = bank.query(&api, &store, &querier, &block, req).unwrap();
        let res: BalanceResponse = from_json(raw).unwrap();
        assert_eq!(res.amount, coin(100, "eth"));

        let req = BankQuery::Balance {
            address: owner.into(),
            denom: "foobar".into(),
        };
        let raw = bank.query(&api, &store, &querier, &block, req).unwrap();
        let res: BalanceResponse = from_json(raw).unwrap();
        assert_eq!(res.amount, coin(0, "foobar"));

        let req = BankQuery::Balance {
            address: rcpt.clone().into(),
            denom: "eth".into(),
        };
        let raw = bank.query(&api, &store, &querier, &block, req).unwrap();
        let res: BalanceResponse = from_json(raw).unwrap();
        assert_eq!(res.amount, coin(0, "eth"));

        // Query total supply of a denom
        let req = BankQuery::Supply {
            denom: "eth".into(),
        };
        let raw = bank.query(&api, &store, &querier, &block, req).unwrap();
        let res: SupplyResponse = from_json(raw).unwrap();
        assert_eq!(res.amount, coin(100, "eth"));

        // Mint tokens for recipient account
        let msg = BankSudo::Mint {
            to_address: rcpt.to_string(),
            amount: norm.clone(),
        };
        bank.sudo(&api, &mut store, &router, &block, msg).unwrap();

        // Check that the recipient account has the expected balance
        #[allow(deprecated)]
        let req = BankQuery::AllBalances {
            address: rcpt.into(),
        };
        let raw = bank.query(&api, &store, &querier, &block, req).unwrap();
        let res: AllBalanceResponse = from_json(raw).unwrap();
        assert_eq!(res.amount, norm);

        // Check that the total supply of a denom is updated
        let req = BankQuery::Supply {
            denom: "eth".into(),
        };
        let raw = bank.query(&api, &store, &querier, &block, req).unwrap();
        let res: SupplyResponse = from_json(raw).unwrap();
        assert_eq!(res.amount, coin(200, "eth"));
    }

    #[test]
    fn send_coins() {
        let api = MockApi::default();
        let mut store = MockStorage::new();
        let block = mock_env().block;
        let router = MockRouter::default();

        let owner = api.addr_make("owner");
        let rcpt = api.addr_make("receiver");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        let rcpt_funds = vec![coin(5, "btc")];

        // set money
        let bank = BankKeeper::new();
        bank.init_balance(&mut store, &owner, init_funds).unwrap();
        bank.init_balance(&mut store, &rcpt, rcpt_funds).unwrap();

        // send both tokens
        let to_send = vec![coin(30, "eth"), coin(5, "btc")];
        let msg = BankMsg::Send {
            to_address: rcpt.clone().into(),
            amount: to_send,
        };
        bank.execute(
            &api,
            &mut store,
            &router,
            &block,
            owner.clone(),
            msg.clone(),
        )
        .unwrap();
        let rich = query_balance(&bank, &api, &store, &owner);
        assert_eq!(vec![coin(15, "btc"), coin(70, "eth")], rich);
        let poor = query_balance(&bank, &api, &store, &rcpt);
        assert_eq!(vec![coin(10, "btc"), coin(30, "eth")], poor);

        // can send from any account with funds
        bank.execute(&api, &mut store, &router, &block, rcpt.clone(), msg)
            .unwrap();

        // cannot send too much
        let msg = BankMsg::Send {
            to_address: rcpt.into(),
            amount: coins(20, "btc"),
        };
        bank.execute(&api, &mut store, &router, &block, owner.clone(), msg)
            .unwrap_err();

        let rich = query_balance(&bank, &api, &store, &owner);
        assert_eq!(vec![coin(15, "btc"), coin(70, "eth")], rich);
    }

    #[test]
    fn burn_coins() {
        let api = MockApi::default();
        let mut store = MockStorage::new();
        let block = mock_env().block;
        let router = MockRouter::default();

        let owner = api.addr_make("owner");
        let rcpt = api.addr_make("recipient");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];

        // set money
        let bank = BankKeeper::new();
        bank.init_balance(&mut store, &owner, init_funds).unwrap();

        // burn both tokens
        let to_burn = vec![coin(30, "eth"), coin(5, "btc")];
        let msg = BankMsg::Burn { amount: to_burn };
        bank.execute(&api, &mut store, &router, &block, owner.clone(), msg)
            .unwrap();
        let rich = query_balance(&bank, &api, &store, &owner);
        assert_eq!(vec![coin(15, "btc"), coin(70, "eth")], rich);

        // cannot burn too much
        let msg = BankMsg::Burn {
            amount: coins(20, "btc"),
        };
        let err = bank
            .execute(&api, &mut store, &router, &block, owner.clone(), msg)
            .unwrap_err();
        assert!(matches!(err.downcast().unwrap(), StdError::Overflow { .. }));

        let rich = query_balance(&bank, &api, &store, &owner);
        assert_eq!(vec![coin(15, "btc"), coin(70, "eth")], rich);

        // cannot burn from empty account
        let msg = BankMsg::Burn {
            amount: coins(1, "btc"),
        };
        let err = bank
            .execute(&api, &mut store, &router, &block, rcpt, msg)
            .unwrap_err();
        assert!(matches!(err.downcast().unwrap(), StdError::Overflow { .. }));
    }

    #[test]
    #[cfg(feature = "cosmwasm_1_3")]
    fn set_get_denom_metadata_should_work() {
        let api = MockApi::default();
        let mut store = MockStorage::new();
        let block = mock_env().block;
        let querier: MockQuerier<Empty> = MockQuerier::new(&[]);
        let bank = BankKeeper::new();
        // set metadata for Ether
        let denom_eth_name = "eth".to_string();
        bank.set_denom_metadata(
            &mut store,
            denom_eth_name.clone(),
            DenomMetadata {
                name: denom_eth_name.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        // query metadata
        let req = BankQuery::DenomMetadata {
            denom: denom_eth_name.clone(),
        };
        let raw = bank.query(&api, &store, &querier, &block, req).unwrap();
        let res: DenomMetadataResponse = from_json(raw).unwrap();
        assert_eq!(res.metadata.name, denom_eth_name);
    }

    #[test]
    #[cfg(feature = "cosmwasm_1_3")]
    fn set_get_all_denom_metadata_should_work() {
        let api = MockApi::default();
        let mut store = MockStorage::new();
        let block = mock_env().block;
        let querier: MockQuerier<Empty> = MockQuerier::new(&[]);
        let bank = BankKeeper::new();
        // set metadata for Bitcoin
        let denom_btc_name = "btc".to_string();
        bank.set_denom_metadata(
            &mut store,
            denom_btc_name.clone(),
            DenomMetadata {
                name: denom_btc_name.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        // set metadata for Ether
        let denom_eth_name = "eth".to_string();
        bank.set_denom_metadata(
            &mut store,
            denom_eth_name.clone(),
            DenomMetadata {
                name: denom_eth_name.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        // query metadata
        let req = BankQuery::AllDenomMetadata { pagination: None };
        let raw = bank.query(&api, &store, &querier, &block, req).unwrap();
        let res: AllDenomMetadataResponse = from_json(raw).unwrap();
        assert_eq!(res.metadata[0].name, denom_btc_name);
        assert_eq!(res.metadata[1].name, denom_eth_name);
    }

    #[test]
    fn fail_on_zero_values() {
        let api = MockApi::default();
        let mut store = MockStorage::new();
        let block = mock_env().block;
        let router = MockRouter::default();

        let owner = api.addr_make("owner");
        let rcpt = api.addr_make("recipient");
        let init_funds = vec![coin(5000, "atom"), coin(100, "eth")];

        // set money
        let bank = BankKeeper::new();
        bank.init_balance(&mut store, &owner, init_funds).unwrap();

        // can send normal amounts
        let msg = BankMsg::Send {
            to_address: rcpt.to_string(),
            amount: coins(100, "atom"),
        };
        bank.execute(&api, &mut store, &router, &block, owner.clone(), msg)
            .unwrap();

        // fails send on no coins
        let msg = BankMsg::Send {
            to_address: rcpt.to_string(),
            amount: vec![],
        };
        bank.execute(&api, &mut store, &router, &block, owner.clone(), msg)
            .unwrap_err();

        // fails send on 0 coins
        let msg = BankMsg::Send {
            to_address: rcpt.to_string(),
            amount: coins(0, "atom"),
        };
        bank.execute(&api, &mut store, &router, &block, owner.clone(), msg)
            .unwrap_err();

        // fails burn on no coins
        let msg = BankMsg::Burn { amount: vec![] };
        bank.execute(&api, &mut store, &router, &block, owner.clone(), msg)
            .unwrap_err();

        // fails burn on 0 coins
        let msg = BankMsg::Burn {
            amount: coins(0, "atom"),
        };
        bank.execute(&api, &mut store, &router, &block, owner, msg)
            .unwrap_err();

        // can mint via sudo
        let msg = BankSudo::Mint {
            to_address: rcpt.to_string(),
            amount: coins(4321, "atom"),
        };
        bank.sudo(&api, &mut store, &router, &block, msg).unwrap();

        // mint fails with 0 tokens
        let msg = BankSudo::Mint {
            to_address: rcpt.to_string(),
            amount: coins(0, "atom"),
        };
        bank.sudo(&api, &mut store, &router, &block, msg)
            .unwrap_err();

        // mint fails with no tokens
        let msg = BankSudo::Mint {
            to_address: rcpt.to_string(),
            amount: vec![],
        };
        bank.sudo(&api, &mut store, &router, &block, msg)
            .unwrap_err();
    }
}
