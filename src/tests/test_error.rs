use crate::error::Error;
use cosmwasm_std::{WasmMsg, WasmQuery};

#[test]
fn instantiating_error_should_work() {
    assert_eq!(
        "Empty attribute key. Value: alpha",
        Error::empty_attribute_key("alpha").to_string()
    );
    assert_eq!(
        "Empty attribute value. Key: beta",
        Error::empty_attribute_value("beta").to_string()
    );
    assert_eq!(
        "Attribute key starts with reserved prefix _: gamma",
        Error::reserved_attribute_key("gamma").to_string()
    );
    assert_eq!(
        "Event type too short: event_type",
        Error::event_type_too_short("event_type").to_string()
    );
    assert_eq!(
        r#"Unsupported wasm query: ContractInfo { contract_addr: "contract1984" }"#,
        Error::unsupported_wasm_query(WasmQuery::ContractInfo {
            contract_addr: "contract1984".to_string()
        })
        .to_string()
    );
    assert_eq!(
        r#"Unsupported wasm message: Migrate { contract_addr: "contract1984", new_code_id: 1984, msg:  }"#,
        Error::unsupported_wasm_message(WasmMsg::Migrate {
            contract_addr: "contract1984".to_string(),
            new_code_id: 1984,
            msg: Default::default(),
        })
        .to_string()
    );
    assert_eq!(
        "code id: invalid",
        Error::invalid_contract_code_id().to_string()
    );
    assert_eq!(
        "code id 53: no such code",
        Error::unregistered_code_id(53).to_string()
    );
    assert_eq!(
        "Contract with this address already exists: contract1984",
        Error::duplicated_contract_address("contract1984").to_string()
    );
}
