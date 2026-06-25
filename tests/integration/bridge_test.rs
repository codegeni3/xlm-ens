#[cfg(test)]
mod bridge_integration {
    use soroban_sdk::{testutils::Address as _, Address, Env, String};
    use xlm_ns_bridge::{BridgeContract, BridgeContractClient, BridgeError};

    fn setup_env() -> (Env, BridgeContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(BridgeContract, ());
        let client = BridgeContractClient::new(&env, &contract_id);
        client.initialize(&Address::generate(&env));
        (env, client)
    }

    fn add_supported_chain(
        client: &BridgeContractClient<'_>,
        env: &Env,
        chain: &str,
        resolver: &str,
    ) {
        client.add_supported_chain(
            &String::from_str(env, chain),
            &String::from_str(env, resolver),
        );
    }

    /// Test covers route registration and exact message generation shape.
    #[test]
    fn test_route_registration_and_message_generation() {
        let (env, client) = setup_env();

        let chain = String::from_str(&env, "ethereum");
        let name = String::from_str(&env, "alice.xlm");

        add_supported_chain(&client, &env, "ethereum", "0xethResolver");
        client.register_chain(&chain);

        let payload = client.build_message(&name, &chain);

        let expected_payload = String::from_str(
            &env,
            r#"{"type":"xlm-ns-resolution","name":"alice.xlm","destination_chain":"ethereum","resolver":"0xethResolver"}"#,
        );
        assert_eq!(payload, expected_payload);
    }

    /// Unsupported destination chains are rejected at route creation time.
    #[test]
    fn test_invalid_chain_registration_rejected() {
        let (env, client) = setup_env();
        let chain = String::from_str(&env, "solana");
        let result = client.try_register_chain(&chain);
        assert!(matches!(result, Err(Ok(BridgeError::UnsupportedChain))));
    }

    /// Malformed route data (such as an invalid name without TLD) is rejected.
    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_malformed_route_data_rejected() {
        let (env, client) = setup_env();
        let chain = String::from_str(&env, "base");
        add_supported_chain(&client, &env, "base", "0xbaseResolver");
        client.register_chain(&chain);

        let malformed_name = String::from_str(&env, "malformed-name");
        client.build_message(&malformed_name, &chain);
    }

    /// Admin can register and list supported destination chains.
    #[test]
    fn test_supported_chain_management() {
        let (env, client) = setup_env();

        add_supported_chain(&client, &env, "base", "0xbaseResolver");
        add_supported_chain(&client, &env, "ethereum", "0xethResolver");

        let chains = client.get_supported_chains();
        assert_eq!(chains.len(), 2);

        let base = String::from_str(&env, "base");
        client.remove_supported_chain(&base);
        assert_eq!(client.get_supported_chains().len(), 1);
    }
}
