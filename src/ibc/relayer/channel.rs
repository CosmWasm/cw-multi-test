use anyhow::Result as AnyResult;
use cosmwasm_std::{from_json, Api, CustomMsg, CustomQuery, IbcEndpoint, IbcOrder, Storage};
use serde::de::DeserializeOwned;

use crate::{
    ibc::{
        types::{Connection, MockIbcQuery},
        IbcPacketRelayingMsg,
    },
    App, AppResponse, Bank, Distribution, Gov, Ibc, Module, Staking, Wasm,
};

use super::get_event_attr_value;

#[derive(Debug)]
pub struct ChannelCreationResult {
    pub init: AppResponse,
    pub r#try: AppResponse,
    pub ack: AppResponse,
    pub confirm: AppResponse,
    pub src_channel: String,
    pub dst_channel: String,
}

pub fn create_connection<
    BankT1,
    ApiT1,
    StorageT1,
    CustomT1,
    WasmT1,
    StakingT1,
    DistrT1,
    IbcT1,
    GovT1,
    BankT2,
    ApiT2,
    StorageT2,
    CustomT2,
    WasmT2,
    StakingT2,
    DistrT2,
    IbcT2,
    GovT2,
>(
    src_app: &mut App<BankT1, ApiT1, StorageT1, CustomT1, WasmT1, StakingT1, DistrT1, IbcT1, GovT1>,
    dst_app: &mut App<BankT2, ApiT2, StorageT2, CustomT2, WasmT2, StakingT2, DistrT2, IbcT2, GovT2>,
) -> AnyResult<(String, String)>
where
    CustomT1::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT1::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT1: Wasm<CustomT1::ExecT, CustomT1::QueryT>,
    BankT1: Bank,
    ApiT1: Api,
    StorageT1: Storage,
    CustomT1: Module,
    StakingT1: Staking,
    DistrT1: Distribution,
    IbcT1: Ibc,
    GovT1: Gov,

    CustomT2::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT2::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT2: Wasm<CustomT2::ExecT, CustomT2::QueryT>,
    BankT2: Bank,
    ApiT2: Api,
    StorageT2: Storage,
    CustomT2: Module,
    StakingT2: Staking,
    DistrT2: Distribution,
    IbcT2: Ibc,
    GovT2: Gov,
{
    let src_connection_msg = IbcPacketRelayingMsg::CreateConnection {
        remote_chain_id: dst_app.block_info().chain_id,
        connection_id: None,
        counterparty_connection_id: None,
    };
    let src_create_response = src_app.sudo(crate::SudoMsg::Ibc(src_connection_msg))?;
    let src_connection =
        get_event_attr_value(&src_create_response, "connection_open", "connection_id")?;

    let dst_connection_msg = IbcPacketRelayingMsg::CreateConnection {
        remote_chain_id: src_app.block_info().chain_id,
        connection_id: None,
        counterparty_connection_id: Some(src_connection.clone()),
    };
    let dst_create_response = dst_app.sudo(crate::SudoMsg::Ibc(dst_connection_msg))?;
    let dst_connection =
        get_event_attr_value(&dst_create_response, "connection_open", "connection_id")?;

    let src_connection_msg = IbcPacketRelayingMsg::CreateConnection {
        remote_chain_id: dst_app.block_info().chain_id,
        connection_id: Some(src_connection.clone()),
        counterparty_connection_id: Some(dst_connection.clone()),
    };
    src_app.sudo(crate::SudoMsg::Ibc(src_connection_msg))?;

    Ok((src_connection, dst_connection))
}
pub fn create_channel<
    BankT1,
    ApiT1,
    StorageT1,
    CustomT1,
    WasmT1,
    StakingT1,
    DistrT1,
    IbcT1,
    GovT1,
    BankT2,
    ApiT2,
    StorageT2,
    CustomT2,
    WasmT2,
    StakingT2,
    DistrT2,
    IbcT2,
    GovT2,
>(
    src_app: &mut App<BankT1, ApiT1, StorageT1, CustomT1, WasmT1, StakingT1, DistrT1, IbcT1, GovT1>,
    dst_app: &mut App<BankT2, ApiT2, StorageT2, CustomT2, WasmT2, StakingT2, DistrT2, IbcT2, GovT2>,
    src_connection_id: String,
    src_port: String,
    dst_port: String,
    version: String,
    order: IbcOrder,
) -> AnyResult<ChannelCreationResult>
where
    CustomT1::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT1::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT1: Wasm<CustomT1::ExecT, CustomT1::QueryT>,
    BankT1: Bank,
    ApiT1: Api,
    StorageT1: Storage,
    CustomT1: Module,
    StakingT1: Staking,
    DistrT1: Distribution,
    IbcT1: Ibc,
    GovT1: Gov,

    CustomT2::ExecT: CustomMsg + DeserializeOwned + 'static,
    CustomT2::QueryT: CustomQuery + DeserializeOwned + 'static,
    WasmT2: Wasm<CustomT2::ExecT, CustomT2::QueryT>,
    BankT2: Bank,
    ApiT2: Api,
    StorageT2: Storage,
    CustomT2: Module,
    StakingT2: Staking,
    DistrT2: Distribution,
    IbcT2: Ibc,
    GovT2: Gov,
{
    let ibc_init_msg = IbcPacketRelayingMsg::OpenChannel {
        local_connection_id: src_connection_id.clone(),
        local_port: src_port.clone(),
        version: version.clone(),
        order: order.clone(),
        counterparty_version: None,
        counterparty_endpoint: IbcEndpoint {
            port_id: dst_port.clone(),
            channel_id: "".to_string(),
        },
    };

    let init_response = src_app.sudo(crate::SudoMsg::Ibc(ibc_init_msg))?;

    log::debug!("Channel init {:?}", init_response);

    // Get the returned version
    let new_version = get_event_attr_value(&init_response, "channel_open_init", "version")?;
    // Get the returned channel id
    let src_channel = get_event_attr_value(&init_response, "channel_open_init", "channel_id")?;

    let counterparty: Connection = from_json(src_app.ibc_query(MockIbcQuery::ConnectedChain {
        connection_id: src_connection_id,
    })?)?;

    let ibc_try_msg = IbcPacketRelayingMsg::OpenChannel {
        local_connection_id: counterparty.counterparty_connection_id.unwrap(),
        local_port: dst_port.clone(),
        version: version.clone(),
        order,
        counterparty_version: Some(new_version),
        counterparty_endpoint: IbcEndpoint {
            port_id: src_port.clone(),
            channel_id: src_channel.clone(),
        },
    };

    let try_response: crate::AppResponse = dst_app.sudo(crate::SudoMsg::Ibc(ibc_try_msg))?;
    log::debug!("Channel try {:?}", try_response);

    // Get the returned version
    let new_version = get_event_attr_value(&try_response, "channel_open_try", "version")?;
    // Get the returned channel id
    let dst_channel = get_event_attr_value(&try_response, "channel_open_try", "channel_id")?;

    let ibc_ack_msg = IbcPacketRelayingMsg::ConnectChannel {
        port_id: src_port.clone(),
        channel_id: src_channel.clone(),
        counterparty_version: Some(new_version.clone()),
        counterparty_endpoint: IbcEndpoint {
            port_id: dst_port.clone(),
            channel_id: dst_channel.clone(),
        },
    };

    let ack_response: crate::AppResponse = src_app.sudo(crate::SudoMsg::Ibc(ibc_ack_msg))?;
    log::debug!("Channel ack {:?}", ack_response);

    let ibc_ack_msg = IbcPacketRelayingMsg::ConnectChannel {
        port_id: dst_port.clone(),
        channel_id: dst_channel.clone(),
        counterparty_version: Some(new_version),
        counterparty_endpoint: IbcEndpoint {
            port_id: src_port.clone(),
            channel_id: src_channel.clone(),
        },
    };

    let confirm_response: crate::AppResponse = dst_app.sudo(crate::SudoMsg::Ibc(ibc_ack_msg))?;
    log::debug!("Channel confirm {:?}", confirm_response);

    Ok(ChannelCreationResult {
        init: init_response,
        r#try: try_response,
        ack: ack_response,
        confirm: confirm_response,
        src_channel,
        dst_channel,
    })
}
