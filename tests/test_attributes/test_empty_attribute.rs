use cosmwasm_std::Empty;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

mod test_contract {
    use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, Event, MessageInfo, Response, StdError};

    pub fn instantiate(
        _deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        _msg: Empty,
    ) -> Result<Response, StdError> {
        Ok(Response::default())
    }

    pub fn execute(
        _deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        _msg: Empty,
    ) -> Result<Response, StdError> {
        Ok(Response::<Empty>::new()
            .add_attribute("city", "    ")
            .add_attribute("street", "")
            .add_event(
                Event::new("location")
                    .add_attribute("longitude", "   ")
                    .add_attribute("latitude", ""),
            ))
    }

    pub fn query(_deps: Deps, _env: Env, _msg: Empty) -> Result<Binary, StdError> {
        Ok(Binary::default())
    }
}

fn contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new_with_empty(
        test_contract::execute,
        test_contract::instantiate,
        test_contract::query,
    ))
}

#[test]
fn empty_string_attribute_should_work() {
    // prepare the blockchain
    let mut app = App::default();

    // prepare address for creator=owner=sender
    let sender_addr = app.api().addr_make("sender");

    // store the contract's code
    let code_id = app.store_code_with_creator(sender_addr.clone(), contract());

    // instantiate the contract
    let contract_addr = app
        .instantiate_contract(
            code_id,
            sender_addr.clone(),
            &Empty {},
            &[],
            "attributed",
            None,
        )
        .unwrap();

    // execute message on the contract, this returns response
    // with attributes having empty string values, which should not fail
    assert!(app
        .execute_contract(sender_addr, contract_addr, &Empty {}, &[])
        .is_ok());
}
