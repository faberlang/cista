//! Fail-closed HTTP transport for the cista.dev registry API.

use std::io::Read;

const MAX_RESPONSE_BYTES: u64 = 64 * 1024 * 1024;

/// Registry HTTP client configuration.
#[derive(Debug, Clone)]
pub struct RegistryHttpClient {
    base_url: String,
    bearer_token: Option<String>,
}

impl RegistryHttpClient {
    /// Construct a client for an HTTP(S) registry origin.
    pub fn new(base_url: &str, bearer_token: Option<&str>) -> Result<Self, String> {
        let base_url = base_url.trim_end_matches('/');
        if !(base_url.starts_with("https://") || base_url.starts_with("http://")) {
            return Err("registry URL must use http:// or https://".to_owned());
        }
        if base_url.starts_with("http://") && bearer_token.is_some() {
            return Err("refusing to send registry credentials over insecure HTTP".to_owned());
        }
        if bearer_token.is_some_and(|token| token.trim().is_empty()) {
            return Err("registry bearer token must not be empty".to_owned());
        }
        Ok(Self {
            base_url: base_url.to_owned(),
            bearer_token: bearer_token.map(str::to_owned),
        })
    }

    /// Fetch a registry API resource, requiring a successful response.
    pub fn get(&self, path: &str) -> Result<Vec<u8>, String> {
        if !path.starts_with('/') || path.starts_with("//") {
            return Err("registry API path must start with exactly one `/`".to_owned());
        }
        let url = format!("{}{}", self.base_url, path);
        let mut request = ureq::get(&url).header("Accept", "application/octet-stream");
        if let Some(token) = &self.bearer_token {
            request = request.header("Authorization", &format!("Bearer {token}"));
        }
        let mut response = request
            .call()
            .map_err(|error| format!("registry request failed for {url}: {error}"))?;
        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .take(MAX_RESPONSE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("failed to read registry response from {url}: {error}"))?;
        if bytes.len() as u64 > MAX_RESPONSE_BYTES {
            return Err(format!(
                "registry response from {url} exceeds {MAX_RESPONSE_BYTES} bytes"
            ));
        }
        Ok(bytes)
    }
}

#[cfg(test)]
#[path = "registry_http_test.rs"]
mod tests;
