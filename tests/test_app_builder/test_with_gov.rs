#[test]
fn building_app_with_custom_gov_should_work() {
    use crate::test_app_builder::MyKeeper;
    use cosmwasm_std::{Empty, GovMsg, VoteOption};
    use cw_multi_test::{no_init, AppBuilder, Executor, Gov};

    type MyGovKeeper = MyKeeper<GovMsg, Empty, Empty>;

    impl Gov for MyGovKeeper {}

    const EXECUTE_MSG: &str = "gov execute called";

    // build custom governance keeper (no query and sudo handling for gov module)
    let gov_keeper = MyGovKeeper::new(EXECUTE_MSG, "", "");

    // build the application with custom gov keeper
    let mut app = AppBuilder::default().with_gov(gov_keeper).build(no_init);

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

#[test]
fn building_app_with_default_gov_vote_should_work() {
    use cosmwasm_std::{GovMsg, VoteOption};
    use cw_multi_test::{no_init, AppBuilder, Executor, IntoAddr};

    // Build the application with always failing governance module.
    let mut app = AppBuilder::default().build(no_init);

    // Prepare sender address.
    let sender_addr = "sender".into_addr();

    // Prepare message for vote.
    let vote_msg = GovMsg::Vote {
        proposal_id: 1,
        option: VoteOption::Yes,
    };

    // Execute vote governance message.
    let response = app.execute(sender_addr, vote_msg.into()).unwrap_err();

    // Always an error is returned.
    assert!(response.to_string().starts_with("Unexpected exec msg Vote"));
}

#[cfg(feature = "cosmwasm_1_2")]
#[test]
fn building_app_with_default_gov_vote_weighted_should_work() {
    use cosmwasm_std::{Decimal, GovMsg, Uint128, VoteOption, WeightedVoteOption};
    use cw_multi_test::{no_init, AppBuilder, Executor, IntoAddr};

    // Build the application with always failing governance module.
    let mut app = AppBuilder::default().build(no_init);

    // Prepare sender address.
    let sender_addr = "sender".into_addr();

    // Prepare message for weighted vote.
    let vote_msg = GovMsg::VoteWeighted {
        proposal_id: 1,
        options: vec![WeightedVoteOption {
            option: VoteOption::Yes,
            weight: Decimal::new(Uint128::new(12)),
        }],
    };

    // Execute weighted vote governance message.
    let response = app.execute(sender_addr, vote_msg.into()).unwrap_err();

    // Always an error is returned.
    assert!(response
        .to_string()
        .starts_with("Unexpected exec msg VoteWeighted"));
}

#[test]
fn building_app_with_failing_gov_vote_should_work() {
    use cosmwasm_std::{GovMsg, VoteOption};
    use cw_multi_test::{no_init, AppBuilder, Executor, GovFailingModule, IntoAddr};

    // Build the application with always failing governance module.
    let mut app = AppBuilder::default()
        .with_gov(GovFailingModule::new())
        .build(no_init);

    // Prepare sender address.
    let sender_addr = "sender".into_addr();

    // Prepare message for vote.
    let vote_msg = GovMsg::Vote {
        proposal_id: 1,
        option: VoteOption::Yes,
    };

    // Execute vote governance message.
    let response = app.execute(sender_addr, vote_msg.into()).unwrap_err();

    // Always an error is returned.
    assert!(response.to_string().starts_with("Unexpected exec msg Vote"));
}

#[cfg(feature = "cosmwasm_1_2")]
#[test]
fn building_app_with_failing_gov_vote_weighted_should_work() {
    use cosmwasm_std::{Decimal, GovMsg, Uint128, VoteOption, WeightedVoteOption};
    use cw_multi_test::{no_init, AppBuilder, Executor, GovFailingModule, IntoAddr};

    // Build the application with always failing governance module.
    let mut app = AppBuilder::default()
        .with_gov(GovFailingModule::new())
        .build(no_init);

    // Prepare sender address.
    let sender_addr = "sender".into_addr();

    // Prepare message for weighted vote.
    let vote_msg = GovMsg::VoteWeighted {
        proposal_id: 1,
        options: vec![WeightedVoteOption {
            option: VoteOption::Yes,
            weight: Decimal::new(Uint128::new(12)),
        }],
    };

    // Execute weighted vote governance message.
    let response = app.execute(sender_addr, vote_msg.into()).unwrap_err();

    // Always an error is returned.
    assert!(response
        .to_string()
        .starts_with("Unexpected exec msg VoteWeighted"));
}

#[test]
fn building_app_with_accepting_gov_vote_should_work() {
    use cosmwasm_std::{GovMsg, VoteOption};
    use cw_multi_test::{no_init, AppBuilder, Executor, GovAcceptingModule, IntoAddr};

    // Build the application with always failing governance module.
    let mut app = AppBuilder::default()
        .with_gov(GovAcceptingModule::new())
        .build(no_init);

    // Prepare sender address.
    let sender_addr = "sender".into_addr();

    // Prepare message for vote.
    let vote_msg = GovMsg::Vote {
        proposal_id: 1,
        option: VoteOption::Yes,
    };

    // Execute vote governance message.
    let response = app.execute(sender_addr, vote_msg.into()).unwrap();

    // Always empty data is returned.
    assert_eq!(None, response.data);
}

#[cfg(feature = "cosmwasm_1_2")]
#[test]
fn building_app_with_accepting_gov_vote_weighted_should_work() {
    use cosmwasm_std::{Decimal, GovMsg, Uint128, VoteOption, WeightedVoteOption};
    use cw_multi_test::{no_init, AppBuilder, Executor, GovAcceptingModule, IntoAddr};

    // Build the application with always failing governance module.
    let mut app = AppBuilder::default()
        .with_gov(GovAcceptingModule::new())
        .build(no_init);

    // Prepare sender address.
    let sender_addr = "sender".into_addr();

    // Prepare message for weighted vote.
    let vote_msg = GovMsg::VoteWeighted {
        proposal_id: 1,
        options: vec![WeightedVoteOption {
            option: VoteOption::Yes,
            weight: Decimal::new(Uint128::new(12)),
        }],
    };

    // Execute weighted vote governance message.
    let response = app.execute(sender_addr, vote_msg.into()).unwrap();

    // Always empty data is returned.
    assert_eq!(None, response.data);
}
