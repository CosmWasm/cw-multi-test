use crate::test_app_builder::MyKeeper;
use cosmwasm_std::{coins, BankMsg, BankQuery};
use cw_multi_test::{no_init, AppBuilder, Bank, BankSudo, Executor};

type MyBankKeeper = MyKeeper<BankMsg, BankQuery, BankSudo>;

impl Bank for MyBankKeeper {}

const EXECUTE_MSG: &str = "bank execute called";
const QUERY_MSG: &str = "bank query called";
const SUDO_MSG: &str = "bank sudo called";

#[test]
fn building_app_with_custom_bank_should_work() {
    // build custom bank keeper
    let bank_keeper = MyBankKeeper::new(EXECUTE_MSG, QUERY_MSG, SUDO_MSG);

    // build the application with custom bank keeper
    let app_builder = AppBuilder::default();
    let mut app = app_builder.with_bank(bank_keeper).build(no_init);

    // prepare additional input data
    let recipient = app.api().addr_make("recipient");
    let denom = "eth";

    // executing bank message should return an error defined in custom keeper
    assert_eq!(
        EXECUTE_MSG,
        app.execute(
            app.api().addr_make("sender"),
            BankMsg::Send {
                to_address: recipient.clone().into(),
                amount: coins(1, denom),
            }
            .into(),
        )
        .unwrap_err()
        .to_string()
    );

    // executing bank sudo should return an error defined in custom keeper
    assert_eq!(
        SUDO_MSG,
        app.sudo(
            BankSudo::Mint {
                to_address: recipient.clone().into(),
                amount: vec![],
            }
            .into()
        )
        .unwrap_err()
        .to_string()
    );

    // executing bank query should return an error defined in custom keeper
    assert_eq!(
        format!("Generic error: Querier contract error: {}", QUERY_MSG),
        app.wrap()
            .query_balance(recipient, denom)
            .unwrap_err()
            .to_string()
    );
}
