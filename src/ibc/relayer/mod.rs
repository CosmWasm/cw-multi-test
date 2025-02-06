use cosmwasm_std::{StdError, StdResult};

use crate::AppResponse;

mod channel;
mod packet;

pub use channel::{create_channel, create_connection, ChannelCreationResult};
pub use packet::{relay_packet, relay_packets_in_tx, RelayPacketResult, RelayingResult};

pub fn get_event_attr_value(
    response: &AppResponse,
    event_type: &str,
    attr_key: &str,
) -> StdResult<String> {
    for event in &response.events {
        if event.ty == event_type {
            for attr in &event.attributes {
                if attr.key == attr_key {
                    return Ok(attr.value.clone());
                }
            }
        }
    }

    Err(StdError::generic_err(format!(
        "event of type {event_type} does not have a value at key {attr_key}"
    )))
}

pub fn has_event(response: &AppResponse, event_type: &str) -> bool {
    for event in &response.events {
        if event.ty == event_type {
            return true;
        }
    }
    false
}

pub fn get_all_event_attr_value(
    response: &AppResponse,
    event: &str,
    attribute: &str,
) -> Vec<String> {
    response
        .events
        .iter()
        .filter(|e| e.ty.eq(event))
        .flat_map(|e| {
            e.attributes
                .iter()
                .filter(|a| a.key.eq(attribute))
                .map(|a| a.value.clone())
        })
        .collect()
}
