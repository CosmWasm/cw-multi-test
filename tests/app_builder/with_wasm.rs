use cw_multi_test::{AppBuilder, WasmKeeper};

#[test]
fn building_app_with_custom_wasm_should_work() {
    let mut initialized = false;
    let app_builder = AppBuilder::default();
    let _ = app_builder
        .with_wasm(WasmKeeper::default())
        .build(|_, _, _| {
            initialized = true;
        });
    assert!(initialized);
}
