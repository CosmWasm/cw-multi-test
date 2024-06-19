use crate::test_app_builder::{MyKeeper, NO_MESSAGE};
use cosmwasm_std::{Empty, GovMsg, VoteOption};
use cw_multi_test::{AppBuilder, Executor, Gov};

type MyGovKeeper = MyKeeper<GovMsg, Empty, Empty>;

impl Gov for MyGovKeeper {}

const EXECUTE_MSG: &str = "gov execute called";

#[test]
fn building_app_with_custom_gov_should_work() {
    // build custom governance keeper (no query and sudo handling for gov module)
    let gov_keeper = MyGovKeeper::new(EXECUTE_MSG, NO_MESSAGE, NO_MESSAGE);

    // build the application with custom gov keeper
    let mut app = AppBuilder::default().with_gov(gov_keeper).build_no_init();

    // prepare addresses
    let sender_addr = app.api().addr_make("sender");

    // executing governance message should return an error defined in custom keeper
    assert_eq!(
        EXECUTE_MSG,
        app.execute(
            sender_addr,
            GovMsg::Vote {
                proposal_id: 1,
                option: VoteOption::Yes,
            }
            .into(),
        )
        .unwrap_err()
        .to_string()
    );
}
