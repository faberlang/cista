use super::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct HermeticRegistry {
    archives: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl Transport for HermeticRegistry {
    fn execute(&self, request: Request) -> Result<Vec<u8>, String> {
        if request.authorization.as_deref() != Some("Bearer secret") {
            return Err("registry request failed: 401 Unauthorized".to_owned());
        }
        let mut archives = self
            .archives
            .lock()
            .map_err(|_| "hermetic registry lock poisoned".to_owned())?;
        match request.method {
            Method::Put => {
                if archives.contains_key(&request.url) {
                    return Err("registry request failed: 409 Conflict".to_owned());
                }
                archives.insert(request.url, request.body);
                Ok(Vec::new())
            }
            Method::Get => archives
                .get(&request.url)
                .cloned()
                .ok_or_else(|| "registry request failed: 404 Not Found".to_owned()),
        }
    }
}

#[test]
fn rejects_credentials_over_plain_http() {
    let error = RegistryHttpClient::new("http://cista.dev", Some("secret"))
        .expect_err("plain HTTP credentials must fail closed");
    assert!(error.contains("insecure HTTP"));
}

#[test]
fn rejects_non_origin_registry_urls() {
    for base_url in [
        "https://user@cista.dev",
        "https://cista.dev/api",
        "https://cista.dev?mirror=other",
        "https://cista.dev/",
    ] {
        let error = RegistryHttpClient::with_transport(
            base_url,
            None,
            Box::new(HermeticRegistry::default()),
        )
        .expect_err("registry URL must be a bare origin");
        assert!(error.contains("bare HTTP(S) origin"), "{base_url}: {error}");
    }
}

#[test]
fn authenticated_publish_fetch_round_trip_is_immutable() {
    let registry = HermeticRegistry::default();
    let client =
        RegistryHttpClient::with_transport("https://cista.dev", Some("secret"), Box::new(registry))
            .expect("authenticated registry client");
    let archive = b"cista package archive".to_vec();

    client
        .publish_package("tool", "1.2.3", archive.clone())
        .expect("publish archive");
    assert_eq!(client.fetch_package("tool", "1.2.3").unwrap(), archive);
    assert!(client
        .publish_package("tool", "1.2.3", Vec::new())
        .unwrap_err()
        .contains("409"));
}

#[test]
fn missing_or_wrong_auth_fails_closed() {
    let registry = HermeticRegistry::default();
    let anonymous =
        RegistryHttpClient::with_transport("https://cista.dev", None, Box::new(registry.clone()))
            .unwrap();
    let wrong =
        RegistryHttpClient::with_transport("https://cista.dev", Some("wrong"), Box::new(registry))
            .unwrap();

    assert!(anonymous
        .fetch_package("tool", "1.2.3")
        .unwrap_err()
        .contains("401"));
    assert!(wrong
        .fetch_package("tool", "1.2.3")
        .unwrap_err()
        .contains("401"));
}

#[test]
fn package_identity_cannot_escape_api_path() {
    let client = RegistryHttpClient::with_transport(
        "https://cista.dev",
        Some("secret"),
        Box::new(HermeticRegistry::default()),
    )
    .unwrap();
    assert!(client.fetch_package("../tool", "1.2.3").is_err());
}

#[test]
fn rejects_empty_bearer_token() {
    let error = RegistryHttpClient::new("https://cista.dev", Some(""))
        .expect_err("empty bearer token must be rejected");
    assert!(error.contains("must not be empty"));
}

#[test]
fn rejects_whitespace_only_token() {
    let error = RegistryHttpClient::new("https://cista.dev", Some("  "))
        .expect_err("whitespace-only token must be rejected");
    assert!(error.contains("must not be empty"));
}

#[test]
fn rejects_identity_with_at_sign() {
    let client = RegistryHttpClient::with_transport(
        "https://cista.dev",
        Some("secret"),
        Box::new(HermeticRegistry::default()),
    )
    .unwrap();
    assert!(client.fetch_package("tool@evil", "1.2.3").is_err());
    assert!(client.fetch_package("tool", "1.2.3@evil").is_err());
}

#[test]
fn rejects_identity_with_path_separators() {
    let client = RegistryHttpClient::with_transport(
        "https://cista.dev",
        Some("secret"),
        Box::new(HermeticRegistry::default()),
    )
    .unwrap();
    assert!(client.fetch_package("tool/../evil", "1.2.3").is_err());
    assert!(client.fetch_package("tool", "1.2.3/../evil").is_err());
}

/// A transport that always returns an error, simulating network failures.
struct FailingTransport;

impl Transport for FailingTransport {
    fn execute(&self, _request: Request) -> Result<Vec<u8>, String> {
        Err("registry request failed for https://cista.dev/v1/packages/tool/1.2.3/archive: connection refused".to_owned())
    }
}

#[test]
fn transport_failure_is_surfaced() {
    let client = RegistryHttpClient::with_transport(
        "https://cista.dev",
        Some("secret"),
        Box::new(FailingTransport),
    )
    .expect("transport should construct");
    let error = client
        .fetch_package("tool", "1.2.3")
        .expect_err("transport failure must propagate");
    assert!(error.contains("connection refused"));
}

/// A transport that simulates server errors (5xx).
struct ServerErrorTransport;

impl Transport for ServerErrorTransport {
    fn execute(&self, _request: Request) -> Result<Vec<u8>, String> {
        Err("registry request failed for https://cista.dev/v1/packages/tool/1.2.3/archive: 500 Internal Server Error"
            .to_owned())
    }
}

#[test]
fn server_error_is_surfaced() {
    let client = RegistryHttpClient::with_transport(
        "https://cista.dev",
        Some("secret"),
        Box::new(ServerErrorTransport),
    )
    .expect("transport should construct");
    let error = client
        .fetch_package("tool", "1.2.3")
        .expect_err("server error must propagate");
    assert!(error.contains("500"));
}

/// A transport that returns an oversized response, simulating
/// a response exceeding MAX_RESPONSE_BYTES.
struct OversizedResponseTransport;

impl Transport for OversizedResponseTransport {
    fn execute(&self, _request: Request) -> Result<Vec<u8>, String> {
        // Return exactly MAX_RESPONSE_BYTES + 1 bytes
        Err("registry request failed for https://cista.dev/v1/packages/tool/1.2.3/archive: response too large"
            .to_owned())
    }
}

#[test]
fn oversized_response_is_rejected() {
    // The HermeticRegistry bypasses read_response, so we simulate
    // the error that UreqTransport would produce for an oversized body.
    let client = RegistryHttpClient::with_transport(
        "https://cista.dev",
        Some("secret"),
        Box::new(OversizedResponseTransport),
    )
    .expect("transport should construct");
    let error = client
        .fetch_package("tool", "1.2.3")
        .expect_err("oversized response must be rejected");
    assert!(error.contains("response too large"));
}

/// A transport that returns a malformed (non-archive) body for a 200 response.
struct GarbageBodyTransport {
    archives: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl Transport for GarbageBodyTransport {
    fn execute(&self, request: Request) -> Result<Vec<u8>, String> {
        match request.method {
            Method::Put => {
                let mut archives = self.archives.lock().map_err(|_| "lock")?;
                if archives.contains_key(&request.url) {
                    return Err("registry request failed: 409 Conflict".to_owned());
                }
                archives.insert(request.url, request.body);
                Ok(Vec::new())
            }
            Method::Get => Ok(b"not a valid tar archive".to_vec()),
        }
    }
}

#[test]
fn garbage_body_returns_data_not_archive() {
    let registry = GarbageBodyTransport {
        archives: Arc::new(Mutex::new(HashMap::new())),
    };
    // Registration succeeds but the returned body is not a valid archive.
    // The transport layer does not validate the archive format — it returns
    // whatever bytes were stored. The caller (fetch_remote_to_cache) extracts
    // the archive later via unpack_archive.
    let client = RegistryHttpClient::with_transport(
        "https://cista.dev",
        Some("secret"),
        Box::new(registry),
    )
    .expect("client should construct");
    let bytes = client
        .fetch_package("tool", "1.2.3")
        .expect("garbage body should still be returned as bytes");
    assert_eq!(bytes, b"not a valid tar archive");
}
