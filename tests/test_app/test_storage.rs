mod documentation {

    #[test]
    fn storage_method_should_work() {
        use cosmwasm_std::Storage;
        use cw_multi_test::App;

        let mut app = App::default();

        let key = b"key";
        let value = b"value";

        app.storage_mut().set(key, value);

        assert_eq!(Some(value.to_vec()), app.storage().get(key));
    }

    #[test]
    fn contract_storage_method_should_work() {
        use cw_multi_test::App;
        use cw_multi_test::IntoAddr;

        let mut app = App::default();

        let key = b"key";
        let value = b"value";

        let contract_addr = "contract".into_addr();

        app.contract_storage_mut(&contract_addr).set(key, value);

        assert_eq!(
            Some(value.to_vec()),
            app.contract_storage(&contract_addr).get(key)
        );
    }

    #[test]
    fn prefixed_storage_method_should_work() {
        use cw_multi_test::App;

        let mut app = App::default();

        let key = b"key";
        let value = b"value";

        let namespace = b"bank";

        app.prefixed_storage_mut(namespace).set(key, value);

        assert_eq!(
            Some(value.to_vec()),
            app.prefixed_storage(namespace).get(key)
        );
    }

    #[test]
    fn prefixed_multilevel_storage_method_should_work() {
        use cw_multi_test::App;

        let mut app = App::default();

        let key = b"key";
        let value = b"value";

        let namespaces = &[b"my-module".as_slice(), b"my-bank".as_slice()];

        app.prefixed_multilevel_storage_mut(namespaces)
            .set(key, value);

        assert_eq!(
            Some(value.to_vec()),
            app.prefixed_multilevel_storage(namespaces).get(key)
        );
    }
}
