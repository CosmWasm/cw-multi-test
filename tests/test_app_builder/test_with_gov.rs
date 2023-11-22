#![cfg(feature = "stargate")]

use crate::test_app_builder::{MyKeeper, NO_MESSAGE};
use cosmwasm_std::{Addr, Empty, GovMsg, VoteOption};
use cw_multi_test::{AppBuilder, Executor, Gov};

type MyGovKeeper = MyKeeper<GovMsg, Empty, Empty>;

impl Gov for MyGovKeeper {}

const EXECUTE_MSG: &str = "gov execute called";

#[test]
fn building_app_with_custom_gov_should_work() {
    // build custom gov keeper (no query and sudo handling for gov)
    let gov_keeper = MyGovKeeper::new(EXECUTE_MSG, NO_MESSAGE, NO_MESSAGE);

    // build the application with custom gov keeper
    let app_builder = AppBuilder::default();
    let mut app = app_builder.with_gov(gov_keeper).build(|_, _, _| {});

    // executing gov message should return an error defined in custom keeper
    assert_eq!(
        EXECUTE_MSG,
        app.execute(
            Addr::unchecked("sender"),
            GovMsg::Vote {
                proposal_id: 1,
                vote: VoteOption::Yes,
            }
            .into(),
        )
        .unwrap_err()
        .to_string()
    );
}
