#[cfg(test)]
mod tests {
    use crate::client::XlmNsClient;
    use crate::errors::SdkError;
    use crate::network;
    use crate::types::{
        RegistrationRequest, RenewalRequest, SubmissionStatus, TextRecordUpdate, TransferRequest,
    };
    use stellar_rpc_client::Client;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn client() -> XlmNsClient {
        XlmNsClient::builder("http://localhost")
            .network_passphrase("Test SDF Network ; September 2015")
            .registry("CDAD...REGISTRY")
            .subdomain("CDAD...SUBDOMAIN")
            .bridge("CDAD...BRIDGE")
            .auction("CDAD...AUCTION")
            .registrar("CDAD...REGISTRAR")
            .resolver("CDAD...RESOLVER")
            .build()
    }

    #[tokio::test]
    async fn renewal_returns_rich_receipt() {
        let receipt = client()
            .renew(RenewalRequest {
                name: "test.xlm".into(),
                additional_years: 2,
                signer: Some("alice".into()),
            })
            .await
            .unwrap();

        assert_eq!(receipt.fee_paid, 21);
        assert_eq!(receipt.additional_years, 2);
        assert_eq!(receipt.submission.status, SubmissionStatus::Submitted);
        assert_eq!(receipt.submission.signer.as_deref(), Some("alice"));
        assert!(receipt.new_expiry > 1_682_200_000);
    }

    #[tokio::test]
    async fn registration_quote_exposes_breakdown() {
        let quote = client().quote_registration("alpha", 3).await.unwrap();
        assert_eq!(quote.label, "alpha");
        assert_eq!(quote.duration_years, 3);
        assert_eq!(quote.fee_breakdown.base_fee, 30);
        assert_eq!(quote.fee_breakdown.network_fee, 1);
        assert_eq!(quote.total_fee, 31);
        assert_eq!(quote.fee_currency, "XLM");
        assert!(quote.contract_id.is_some());
    }

    #[tokio::test]
    async fn registration_receipt_carries_submission_metadata() {
        let receipt = client()
            .register(RegistrationRequest {
                label: "beta".into(),
                owner: "GDRA...OWNER".into(),
                duration_years: 1,
                signer: Some("treasury".into()),
            })
            .await
            .unwrap();

        assert_eq!(receipt.name, "beta.xlm");
        assert_eq!(receipt.duration_years, 1);
        assert_eq!(receipt.fee_paid, 11);
        assert_eq!(receipt.submission.signer.as_deref(), Some("treasury"));
        assert!(receipt.submission.network_passphrase.is_some());
    }

    #[tokio::test]
    async fn reverse_resolution_rejects_empty_address() {
        assert!(client().reverse_resolve("").await.is_err());
    }

    #[tokio::test]
    async fn text_record_round_trip() {
        let client = client();
        let record = client.get_text_record("foo.xlm", "url").await.unwrap();
        assert_eq!(record.name, "foo.xlm");
        assert_eq!(record.key, "url");

        let submission = client
            .set_text_record(TextRecordUpdate {
                name: "foo.xlm".into(),
                key: "url".into(),
                value: Some("https://example.xyz".into()),
                signer: Some("owner".into()),
            })
            .await
            .unwrap();
        assert_eq!(submission.status, SubmissionStatus::Submitted);
        assert_eq!(submission.signer.as_deref(), Some("owner"));
    }

    #[tokio::test]
    async fn transfer_returns_submission() {
        let submission = client()
            .transfer(TransferRequest {
                name: "foo.xlm".into(),
                new_owner: "GDRA...NEW".into(),
                signer: Some("alice".into()),
            })
            .await
            .unwrap();
        assert_eq!(submission.status, SubmissionStatus::Submitted);
        assert_eq!(submission.signer.as_deref(), Some("alice"));
    }

    #[tokio::test]
    async fn registry_metadata_returns_typed_record() {
        let metadata = client().get_registry_metadata("alice.xlm").await.unwrap();
        assert_eq!(metadata.owner, "GDRA...OWNER");
        assert!(metadata.expires_at > 0);
        assert!(metadata.resolver.is_some());
    }

    #[tokio::test]
    async fn owner_portfolio_returns_vec() {
        let portfolio = client()
            .get_owner_portfolio("GDRA...OWNER")
            .await
            .unwrap();
        assert!(!portfolio.is_empty());
        assert_eq!(portfolio[0].owner, "GDRA...OWNER");
    }

    #[tokio::test]
    async fn auction_state_returns_typed_data() {
        let state = client().get_auction_state("active.xlm").await.unwrap();
        assert_eq!(state.highest_bid, 150);
        assert!(state.end_time > 0);
    }

    #[tokio::test]
    async fn auction_state_handles_not_found() {
        use crate::errors::SdkError;
        use crate::errors::ContractErrorCode;
        let result = client().get_auction_state("missing.xlm").await;
        match result {
            Err(SdkError::ContractError(ContractErrorCode::NameNotFound)) => {},
            _ => panic!("Expected NameNotFound error"),
        }
    }

    #[tokio::test]
    async fn resolver_primary_name_returns_option() {
        let name = client().get_primary_name("GDRA...ADDR").await.unwrap();
        assert_eq!(name, Some("primary.xlm".to_string()));
    }

    #[tokio::test]
    async fn resolver_text_records_returns_hashmap() {
        let records = client().get_text_records("alice.xlm").await.unwrap();
        assert!(records.contains_key("url"));
        assert_eq!(records.get("url").unwrap(), "https://alice.xlm");
    }

    #[tokio::test]
    async fn builder_default_config_is_applied() {
        let client = client();
        assert_eq!(client.config.timeout, crate::config::DEFAULT_TIMEOUT);
        assert!(client.config.user_agent.starts_with("xlm-ns-sdk/"));
    }

    #[tokio::test]
    async fn builder_accepts_custom_config() {
        use crate::config::ClientConfig;
        use std::time::Duration;

        let client = XlmNsClient::builder("http://localhost")
            .registry("CDAD...REGISTRY")
            .config(
                ClientConfig::default()
                    .with_timeout(Duration::from_secs(2))
                    .with_max_retries(0)
                    .with_user_agent("integration-test/1.0"),
            )
            .build();

        assert_eq!(client.config.timeout, Duration::from_secs(2));
        assert_eq!(client.config.retry.max_retries, 0);
        assert_eq!(client.config.user_agent, "integration-test/1.0");
    }

    #[test]
    fn error_decoding_works() {
        use crate::errors::decode_error;
        use crate::errors::ContractErrorCode;
        assert_eq!(decode_error(1), ContractErrorCode::NameNotFound);
        assert_eq!(decode_error(2), ContractErrorCode::NotOwner);
        assert_eq!(decode_error(99), ContractErrorCode::Other);
    }

    #[tokio::test]
    async fn test_verify_passphrase_happy_path() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "passphrase": "Test SDF Network ; September 2015",
                    "protocolVersion": 21
                }
            })))
            .mount(&mock_server)
            .await;
        let http_client = Client::new(&mock_server.uri()).unwrap();

        let result = network::verify_network_passphrase(
            "Test SDF Network ; September 2015",
            &mock_server.uri(),
            &http_client,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_passphrase_mismatch_returns_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": {
                    "passphrase": "Public Global Stellar Network ; September 2015",
                    "protocolVersion": 21
                }
            })))
            .mount(&mock_server)
            .await;
        let http_client = Client::new(&mock_server.uri()).unwrap();

        let result = network::verify_network_passphrase(
            "Test SDF Network ; September 2015",
            &mock_server.uri(),
            &http_client,
        )
        .await;

        let err = result.unwrap_err();
        match err {
            SdkError::NetworkPassphraseMismatch {
                configured,
                rpc_reported,
            } => {
                assert_eq!(configured, "Test SDF Network ; September 2015");
                assert_eq!(
                    rpc_reported,
                    "Public Global Stellar Network ; September 2015"
                );
            }
            _ => panic!("wrong error variant"),
        }
    }

    #[tokio::test]
    async fn test_verify_passphrase_transport_failure() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;
        let http_client = Client::new(&mock_server.uri()).unwrap();

        let result = network::verify_network_passphrase(
            "Test SDF Network ; September 2015",
            &mock_server.uri(),
            &http_client,
        )
        .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SdkError::NetworkPassphraseMismatch { .. } => {
                panic!("should be a transport error, not a mismatch")
            }
            _ => {}
        }
    }

    #[test]
    fn test_verify_transaction_passphrase_mismatch() {
        let result = network::verify_transaction_passphrase(
            "Test SDF Network ; September 2015",
            "Public Global Stellar Network ; September 2015",
        );

        let err = result.unwrap_err();
        match err {
            SdkError::TransactionPassphraseMismatch {
                configured,
                in_transaction,
            } => {
                assert_eq!(configured, "Test SDF Network ; September 2015");
                assert_eq!(
                    in_transaction,
                    "Public Global Stellar Network ; September 2015"
                );
            }
            _ => panic!("wrong error variant"),
        }
    }
}
