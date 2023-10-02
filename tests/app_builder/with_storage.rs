use cosmwasm_std::{Order, Record, Storage};
use cw_multi_test::AppBuilder;

struct MyStorage {}

impl Storage for MyStorage {
    fn get(&self, _key: &[u8]) -> Option<Vec<u8>> {
        todo!()
    }

    fn range<'a>(
        &'a self,
        _start: Option<&[u8]>,
        _end: Option<&[u8]>,
        _order: Order,
    ) -> Box<dyn Iterator<Item = Record> + 'a> {
        todo!()
    }

    fn set(&mut self, _key: &[u8], _value: &[u8]) {
        todo!()
    }

    fn remove(&mut self, _key: &[u8]) {
        todo!()
    }
}

#[test]
fn building_app_with_custom_storage_should_work() {
    let app_builder = AppBuilder::default();
    let _ = app_builder.with_storage(MyStorage {}).build(|_, _, _| {});
}
