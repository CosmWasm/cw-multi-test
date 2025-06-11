use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response};
use cw_multi_test::error::AnyResult;
use cw_multi_test::Contract;

struct CustomWrapper {}

impl Contract<Empty> for CustomWrapper {
    fn instantiate(
        &self,
        deps: DepsMut<Empty>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<Empty>> {
        let _ = (deps, env, info, msg);
        unimplemented!()
    }

    fn execute(
        &self,
        deps: DepsMut<Empty>,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<Empty>> {
        let _ = (deps, env, info, msg);
        unimplemented!()
    }

    fn query(&self, deps: Deps<Empty>, env: Env, msg: Vec<u8>) -> AnyResult<Binary> {
        let _ = (deps, env, msg);
        unimplemented!()
    }

    fn reply(&self, deps: DepsMut<Empty>, env: Env, msg: Reply) -> AnyResult<Response<Empty>> {
        let _ = (deps, env, msg);
        unimplemented!()
    }

    fn sudo(&self, deps: DepsMut<Empty>, env: Env, msg: Vec<u8>) -> AnyResult<Response<Empty>> {
        let _ = (deps, env, msg);
        unimplemented!()
    }

    fn migrate(&self, deps: DepsMut<Empty>, env: Env, msg: Vec<u8>) -> AnyResult<Response<Empty>> {
        let _ = (deps, env, msg);
        unimplemented!()
    }
}

#[test]
fn creating_custom_wrapper_should_work() {
    let wrapper = CustomWrapper {};
    assert_eq!(None, wrapper.checksum());
}
