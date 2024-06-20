#![cfg(test)]

mod test_app;
mod test_custom_handler;
mod test_error;
#[cfg(feature = "stargate")]
mod test_gov;
#[cfg(feature = "stargate")]
mod test_ibc;
#[cfg(feature = "stargate")]
mod test_stargate;
