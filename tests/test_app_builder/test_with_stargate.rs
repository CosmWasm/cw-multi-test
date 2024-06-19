use anyhow::bail;
use cosmwasm_std::{
    Addr, AnyMsg, Api, Binary, BlockInfo, CosmosMsg, CustomMsg, CustomQuery, Empty, Event,
    GrpcQuery, Querier, QueryRequest, Storage,
};
use cw_multi_test::error::AnyResult;
use cw_multi_test::{
    no_init, AppBuilder, AppResponse, CosmosRouter, Executor, Stargate, StargateAccepting,
    StargateFailing,
};
use serde::de::DeserializeOwned;

const MSG_STARGATE_EXECUTE: &str = "stargate execute called";
const MSG_STARGATE_QUERY: &str = "stargate query called";
const MSG_ANY_EXECUTE: &str = "any execute called";
const MSG_GRPC_QUERY: &str = "grpc query called";

struct StargateKeeper;

impl Stargate for StargateKeeper {
    fn execute_stargate<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _sender: Addr,
        _type_url: String,
        _value: Binary,
    ) -> AnyResult<AppResponse>
    where
        ExecC: CustomMsg + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        bail!(MSG_STARGATE_EXECUTE)
    }

    fn query_stargate(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _path: String,
        _data: Binary,
    ) -> AnyResult<Binary> {
        bail!(MSG_STARGATE_QUERY)
    }

    fn execute_any<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _sender: Addr,
        _msg: AnyMsg,
    ) -> AnyResult<AppResponse>
    where
        ExecC: CustomMsg + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        bail!(MSG_ANY_EXECUTE)
    }

    fn query_grpc(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _request: GrpcQuery,
    ) -> AnyResult<Binary> {
        bail!(MSG_GRPC_QUERY)
    }
}

#[test]
fn building_app_with_custom_stargate_should_work() {
    // build the application with custom stargate keeper
    let app_builder = AppBuilder::default();
    let mut app = app_builder.with_stargate(StargateKeeper).build(no_init);

    // prepare user addresses
    let sender_addr = app.api().addr_make("sender");

    // executing `stargate` message should return an error defined in custom stargate keeper
    #[allow(deprecated)]
    let msg = CosmosMsg::Stargate {
        type_url: "test".to_string(),
        value: Default::default(),
    };
    assert_eq!(
        app.execute(sender_addr, msg).unwrap_err().to_string(),
        MSG_STARGATE_EXECUTE,
    );

    // executing `stargate` query should return an error defined in custom stargate keeper
    #[allow(deprecated)]
    let request: QueryRequest<Empty> = QueryRequest::Stargate {
        path: "test".to_string(),
        data: Default::default(),
    };
    assert!(app
        .wrap()
        .query::<Empty>(&request)
        .unwrap_err()
        .to_string()
        .ends_with(MSG_STARGATE_QUERY));
}

#[test]
#[cfg(feature = "cosmwasm_2_0")]
fn building_app_with_custom_any_grpc_should_work() {
    // build the application with custom stargate keeper
    let mut app = AppBuilder::default()
        .with_stargate(StargateKeeper)
        .build(no_init);

    // prepare user addresses
    let sender_addr = app.api().addr_make("sender");

    // executing `any` message should return an error defined in custom stargate keeper
    let msg = CosmosMsg::Any(AnyMsg {
        type_url: "test".to_string(),
        value: Default::default(),
    });
    assert_eq!(
        app.execute(sender_addr, msg).unwrap_err().to_string(),
        MSG_ANY_EXECUTE,
    );

    // executing `grpc` query should return an error defined in custom stargate keeper
    let request: QueryRequest<Empty> = QueryRequest::Grpc(GrpcQuery {
        path: "test".to_string(),
        data: Default::default(),
    });
    assert!(app
        .wrap()
        .query::<Empty>(&request)
        .unwrap_err()
        .to_string()
        .ends_with(MSG_GRPC_QUERY));
}

#[test]
fn building_app_with_accepting_stargate_should_work() {
    let mut app = AppBuilder::default()
        .with_stargate(StargateAccepting)
        .build(no_init);

    // prepare user addresses
    let sender_addr = app.api().addr_make("sender");

    // executing `stargate` query should success and return empty values
    #[allow(deprecated)]
    let msg = CosmosMsg::Stargate {
        type_url: "test".to_string(),
        value: Default::default(),
    };
    let AppResponse { events, data } = app.execute(sender_addr, msg).unwrap();
    assert_eq!(events, Vec::<Event>::new());
    assert_eq!(data, None);

    // executing `stargate` query should success and return Empty message
    #[allow(deprecated)]
    let request: QueryRequest<Empty> = QueryRequest::Stargate {
        path: "test".to_string(),
        data: Default::default(),
    };
    assert_eq!(app.wrap().query::<Empty>(&request).unwrap(), Empty {});
}

#[test]
#[cfg(feature = "cosmwasm_2_0")]
fn building_app_with_accepting_any_grpc_should_work() {
    let app_builder = AppBuilder::default();
    let mut app = app_builder.with_stargate(StargateAccepting).build(no_init);

    // prepare user addresses
    let sender_addr = app.api().addr_make("sender");

    use cosmwasm_std::to_json_vec;

    // executing `any` message should success and return empty values
    let msg = CosmosMsg::Any(AnyMsg {
        type_url: "test".to_string(),
        value: Default::default(),
    });
    let AppResponse { events, data } = app.execute(sender_addr, msg).unwrap();
    assert_eq!(events, Vec::<Event>::new());
    assert_eq!(data, None);

    // executing `grpc` query should success and return empty binary
    let request: QueryRequest<Empty> = QueryRequest::Grpc(GrpcQuery {
        path: "test".to_string(),
        data: Default::default(),
    });
    assert_eq!(
        app.wrap()
            .raw_query(to_json_vec(&request).unwrap().as_slice())
            .unwrap()
            .unwrap(),
        Binary::default()
    );
}

#[test]
fn default_failing_stargate_should_work() {
    let app_builder = AppBuilder::default();
    let mut app = app_builder.with_stargate(StargateFailing).build(no_init);

    // prepare user addresses
    let sender_addr = app.api().addr_make("sender");

    #[allow(deprecated)]
    let msg = CosmosMsg::Stargate {
        type_url: "test".to_string(),
        value: Default::default(),
    };
    assert!(app
        .execute(sender_addr, msg)
        .unwrap_err()
        .to_string()
        .starts_with("Unexpected stargate execute"));

    #[allow(deprecated)]
    let request: QueryRequest<Empty> = QueryRequest::Stargate {
        path: "test".to_string(),
        data: Default::default(),
    };
    assert!(app
        .wrap()
        .query::<Empty>(&request)
        .unwrap_err()
        .to_string()
        .contains("Unexpected stargate query"));
}

#[test]
#[cfg(feature = "cosmwasm_2_0")]
fn default_failing_any_grpc_should_work() {
    let mut app = AppBuilder::default()
        .with_stargate(StargateFailing)
        .build(no_init);

    // prepare user addresses
    let sender_addr = app.api().addr_make("sender");

    use cosmwasm_std::to_json_vec;

    let msg = CosmosMsg::Any(AnyMsg {
        type_url: "test".to_string(),
        value: Default::default(),
    });
    assert!(app
        .execute(sender_addr, msg)
        .unwrap_err()
        .to_string()
        .starts_with("Unexpected any execute"));

    let request: QueryRequest<Empty> = QueryRequest::Grpc(GrpcQuery {
        path: "test".to_string(),
        data: Default::default(),
    });
    assert!(app
        .wrap()
        .raw_query(to_json_vec(&request).unwrap().as_slice())
        .unwrap()
        .unwrap_err()
        .starts_with("Unexpected grpc query"));
}
