use crate::error::*;
use cosmwasm_std::{WasmMsg, WasmQuery};

#[test]
fn instantiating_error_should_work() {
    assert_eq!(
        "Empty attribute key. Value: alpha",
        empty_attribute_key("alpha").to_string()
    );
    assert_eq!(
        "Attribute key starts with reserved prefix _: gamma",
        reserved_attribute_key("gamma").to_string()
    );
    assert_eq!(
        "Event type too short: event_type",
        event_type_too_short("event_type").to_string()
    );
    assert_eq!(
        r#"Unsupported wasm query: ContractInfo { contract_addr: "contract1984" }"#,
        unsupported_wasm_query(WasmQuery::ContractInfo {
            contract_addr: "contract1984".to_string()
        })
        .to_string()
    );
    assert_eq!(
        r#"Unsupported wasm message: Migrate { contract_addr: "contract1984", new_code_id: 1984, msg:  }"#,
        unsupported_wasm_message(WasmMsg::Migrate {
            contract_addr: "contract1984".to_string(),
            new_code_id: 1984,
            msg: Default::default(),
        })
        .to_string()
    );
    assert_eq!("code id: invalid", invalid_code_id().to_string());
    assert_eq!(
        "code id 53: no such code",
        unregistered_code_id(53).to_string()
    );
    assert_eq!(
        "Contract with this address already exists: contract1984",
        duplicated_contract_address("contract1984").to_string()
    );
}
