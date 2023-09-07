fn main() {
    //TODO Remove the following block when publishing version 1.0 of Multi Test.
    //     Remove this file also when the main function will be then empty.
    {
        #[cfg(feature = "multitest_api_1_0")]
        println!("cargo:warning=Feature \"multitest_api_1_0\" is experimental, DO NOT enable it, unless you want to test Multi Test 1.0 API!");
    }
}
