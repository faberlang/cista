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
