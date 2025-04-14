use cosmwasm_std::DistributionMsg;
use cw_multi_test::{App, Executor, IntoAddr};

#[test]
fn querying_withdraw_address_should_work() {
    // Prepare the delegator address.
    let delegator_addr = "delegator".into_addr();
    // Prepare the address for staking rewards.
    let withdraw_address = "rewards".into_addr();
    // Create a chain with default settings.
    let mut app = App::default();

    // Before changing withdraw address, the queried one should be equal to the delegator address.
    assert_eq!(
        delegator_addr.as_str(),
        app.wrap()
            .query_delegator_withdraw_address(delegator_addr.clone())
            .unwrap()
            .as_str()
    );

    // Change withdraw address for specified delegator.
    app.execute(
        delegator_addr.clone(),
        DistributionMsg::SetWithdrawAddress {
            address: withdraw_address.to_string(),
        }
        .into(),
    )
    .unwrap();

    // Queried withdraw address should be equal to the one set by delegator.
    assert_eq!(
        withdraw_address.as_str(),
        app.wrap()
            .query_delegator_withdraw_address(delegator_addr)
            .unwrap()
            .as_str()
    );
}
