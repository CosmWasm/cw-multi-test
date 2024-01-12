// We create Polytone Contract Wrappers

use cosmwasm_std::{Addr, ContractInfoResponse, Empty, Never, QueryRequest, WasmQuery};

use crate::{
    ibc::{
        addresses::MockAddressGenerator,
        relayer::{create_channel, create_connection, get_event_attr_value, relay_packets_in_tx},
        simple_ibc::IbcSimpleModule,
    },
    AppBuilder, ContractWrapper, Executor, FailingModule, WasmKeeper, addons::MockApiBech32,
};

use anyhow::Result as AnyResult;
use cosmwasm_std::{Api, IbcOrder, Storage};

use crate::{App, Bank, Distribution, Gov, Ibc, Staking};

use polytone::callbacks::{Callback, ExecutionResponse};

type PolytoneNoteType = ContractWrapper<
    polytone_note::msg::ExecuteMsg,
    polytone_note::msg::InstantiateMsg,
    polytone_note::msg::QueryMsg,
    polytone_note::error::ContractError,
    polytone_note::error::ContractError,
    cosmwasm_std::StdError,
    cosmwasm_std::Empty,
    cosmwasm_std::Empty,
    cosmwasm_std::Empty,
    anyhow::Error,
    polytone_note::error::ContractError,
    cosmwasm_std::Empty,
    anyhow::Error,
    polytone_note::error::ContractError,
    polytone_note::error::ContractError,
    polytone_note::error::ContractError,
    Never,
    polytone_note::error::ContractError,
    polytone_note::error::ContractError,
>;

type PolytoneVoiceType = ContractWrapper<
    polytone_voice::msg::ExecuteMsg,
    polytone_voice::msg::InstantiateMsg,
    polytone_voice::msg::QueryMsg,
    polytone_voice::error::ContractError,
    polytone_voice::error::ContractError,
    cosmwasm_std::StdError,
    cosmwasm_std::Empty,
    cosmwasm_std::Empty,
    cosmwasm_std::Empty,
    anyhow::Error,
    polytone_voice::error::ContractError,
    cosmwasm_std::Empty,
    anyhow::Error,
    polytone_voice::error::ContractError,
    polytone_voice::error::ContractError,
    polytone_voice::error::ContractError,
    Never,
    polytone_voice::error::ContractError,
    polytone_voice::error::ContractError,
>;

type PolytoneProxyType = ContractWrapper<
    polytone_proxy::msg::ExecuteMsg,
    polytone_proxy::msg::InstantiateMsg,
    polytone_proxy::msg::QueryMsg,
    polytone_proxy::error::ContractError,
    polytone_proxy::error::ContractError,
    cosmwasm_std::StdError,
    cosmwasm_std::Empty,
    cosmwasm_std::Empty,
    cosmwasm_std::Empty,
    anyhow::Error,
    polytone_proxy::error::ContractError,
>;

fn polytone_note() -> PolytoneNoteType {
    ContractWrapper::new(
        polytone_note::contract::execute,
        polytone_note::contract::instantiate,
        polytone_note::contract::query,
    )
    .with_reply(polytone_note::ibc::reply)
    .with_ibc(
        polytone_note::ibc::ibc_channel_open,
        polytone_note::ibc::ibc_channel_connect,
        polytone_note::ibc::ibc_channel_close,
        polytone_note::ibc::ibc_packet_receive,
        polytone_note::ibc::ibc_packet_ack,
        polytone_note::ibc::ibc_packet_timeout,
    )
}

fn polytone_voice() -> PolytoneVoiceType {
    ContractWrapper::new(
        polytone_voice::contract::execute,
        polytone_voice::contract::instantiate,
        polytone_voice::contract::query,
    )
    .with_reply(polytone_voice::ibc::reply)
    .with_ibc(
        polytone_voice::ibc::ibc_channel_open,
        polytone_voice::ibc::ibc_channel_connect,
        polytone_voice::ibc::ibc_channel_close,
        polytone_voice::ibc::ibc_packet_receive,
        polytone_voice::ibc::ibc_packet_ack,
        polytone_voice::ibc::ibc_packet_timeout,
    )
}

fn polytone_proxy() -> PolytoneProxyType {
    ContractWrapper::new(
        polytone_proxy::contract::execute,
        polytone_proxy::contract::instantiate,
        polytone_proxy::contract::query,
    )
    .with_reply(polytone_proxy::contract::reply)
}

pub const MAX_BLOCK_GAS: u64 = 100_000_000;
pub const ADMIN: &str = "admin";

pub type AppType<BankT1, ApiT1, StorageT1, StakingT1, DistrT1, IbcT1, GovT1> = App<
    BankT1,
    ApiT1,
    StorageT1,
    FailingModule<Empty, Empty, Empty>,
    WasmKeeper<Empty, Empty>,
    StakingT1,
    DistrT1,
    IbcT1,
    GovT1,
>;

pub type DeployReturn<BankT1, ApiT1, StorageT1, StakingT1, DistrT1, IbcT1, GovT1> = AnyResult<(
    AppType<BankT1, ApiT1, StorageT1, StakingT1, DistrT1, IbcT1, GovT1>,
    Addr,
    Addr,
    u64,
)>;

pub fn deploy_polytone<BankT1, ApiT1, StorageT1, StakingT1, DistrT1, IbcT1, GovT1>(
    mut app: AppType<BankT1, ApiT1, StorageT1, StakingT1, DistrT1, IbcT1, GovT1>,
) -> DeployReturn<BankT1, ApiT1, StorageT1, StakingT1, DistrT1, IbcT1, GovT1>
where
    BankT1: Bank,
    ApiT1: Api,
    StorageT1: Storage,
    StakingT1: Staking,
    DistrT1: Distribution,
    IbcT1: Ibc,
    GovT1: Gov,
{
    let admin = Addr::unchecked(ADMIN);
    let note = Box::new(polytone_note());
    let voice = Box::new(polytone_voice());
    let proxy = Box::new(polytone_proxy());
    let note_code_id = app.store_code(note);
    let voice_code_id = app.store_code(voice);
    let proxy_code_id = app.store_code(proxy);

    let note_address = app.instantiate_contract(
        note_code_id,
        admin.clone(),
        &polytone_note::msg::InstantiateMsg {
            pair: None,
            block_max_gas: MAX_BLOCK_GAS.into(),
        },
        &[],
        "note_1".to_string(),
        None,
    )?;

    let voice_address = app.instantiate_contract(
        voice_code_id,
        admin,
        &polytone_voice::msg::InstantiateMsg {
            block_max_gas: MAX_BLOCK_GAS.into(),
            proxy_code_id: proxy_code_id.into(),
        },
        &[],
        "note_1".to_string(),
        None,
    )?;

    Ok((app, note_address, voice_address, proxy_code_id))
}

#[test]
fn polytone() -> anyhow::Result<()> {
    let sender = Addr::unchecked("sender");

    // prepare wasm module with custom address generator
    let wasm_keeper_1: WasmKeeper<Empty, Empty> =
        WasmKeeper::new().with_address_generator(MockAddressGenerator);
    let wasm_keeper_2: WasmKeeper<Empty, Empty> =
        WasmKeeper::new().with_address_generator(MockAddressGenerator);
    // We mint some funds to the owner
    let mut app1 = AppBuilder::default()
        .with_ibc(IbcSimpleModule)
        .with_api(MockApiBech32::new("local"))
        .with_wasm(wasm_keeper_1)
        .build(|_, _, _| {});
    let mut app2 = AppBuilder::default()
        .with_ibc(IbcSimpleModule)
        .with_api(MockApiBech32::new("remote"))
        .with_wasm(wasm_keeper_2)
        .build(|_, _, _| {});

    // We start by uploading the contracts and instantiating them
    let note1: Addr;
    let _note2: Addr;
    let _voice1: Addr;
    let voice2: Addr;
    let _proxy_code_id1: u64;
    let proxy_code_id2: u64;
    (app1, note1, _voice1, _proxy_code_id1) = deploy_polytone(app1)?;
    (app2, _note2, voice2, proxy_code_id2) = deploy_polytone(app2)?;

    // Now, we create a channel between the 2 contracts

    let port1 = format!("wasm.{}", note1);
    let port2 = format!("wasm.{}", voice2);

    let (src_connection_id, _) = create_connection(&mut app1, &mut app2)?;

    // We start by creating channels
    create_channel(
        &mut app1,
        &mut app2,
        src_connection_id,
        port1.clone(),
        port2,
        "polytone-1".to_string(),
        IbcOrder::Unordered,
    )?;

    // We send a simple empty execute message to the note
    let send_response = app1.execute_contract(
        sender,
        note1,
        &polytone_note::msg::ExecuteMsg::Execute {
            msgs: vec![],
            callback: None,
            timeout_seconds: 100_000_000u64.into(),
        },
        &[],
    )?;

    // We relaying all packets found in the transaction
    let packet_txs = relay_packets_in_tx(&mut app1, &mut app2, send_response)?;

    assert_eq!(packet_txs.len(), 1);

    println!("{:?}", packet_txs);
    let contract_addr = get_event_attr_value(&packet_txs[0].0, "instantiate", "_contract_address")?;

    // We test if the proxy is instantiated on app2
    let test: ContractInfoResponse =
        app2.wrap()
            .query(&QueryRequest::Wasm(WasmQuery::ContractInfo {
                contract_addr: contract_addr.clone(),
            }))?;
    assert_eq!(test.code_id, proxy_code_id2);

    // Assert the polytone result (executed_by field of the ack)
    let ack: Callback = serde_json::from_str(&get_event_attr_value(
        &packet_txs[0].0,
        "write_acknowledgement",
        "packet_ack",
    )?)?;

    match ack {
        Callback::Execute(Ok(ExecutionResponse {
            executed_by,
            result,
        })) => {
            assert_eq!(executed_by, contract_addr);
            assert!(result.is_empty());
        }
        _ => panic!("Wrong acknowledgement, {:?}", ack),
    }

    Ok(())
}
