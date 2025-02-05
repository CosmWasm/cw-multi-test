mod documentation {
    use cosmwasm_std::Storage;

    #[test]
    fn storage_method_should_work() {
        use cw_multi_test::App;

        let mut app = App::default();

        let key = b"key";
        let value = b"value";

        app.storage_mut().set(key, value);

        assert_eq!(Some(value.to_vec()), app.storage().get(key));
    }
}
