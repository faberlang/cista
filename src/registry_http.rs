//! Fail-closed HTTP transport for the cista.dev registry API.

use std::io::Read;

const MAX_RESPONSE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
enum Method {
    Get,
    Put,
}

struct Request {
    method: Method,
    url: String,
    authorization: Option<String>,
    body: Vec<u8>,
}

trait Transport: Send + Sync {
    fn execute(&self, request: Request) -> Result<Vec<u8>, String>;
}

struct UreqTransport;

impl Transport for UreqTransport {
    fn execute(&self, request: Request) -> Result<Vec<u8>, String> {
        let response = match request.method {
            Method::Get => {
                let mut builder =
                    ureq::get(&request.url).header("Accept", "application/octet-stream");
                if let Some(authorization) = &request.authorization {
                    builder = builder.header("Authorization", authorization);
                }
                builder.call()
            }
            Method::Put => {
                let mut builder = ureq::put(&request.url)
                    .header("Accept", "application/octet-stream")
                    .header("Content-Type", "application/octet-stream");
                if let Some(authorization) = &request.authorization {
                    builder = builder.header("Authorization", authorization);
                }
                builder.send(&request.body)
            }
        }
        .map_err(|error| format!("registry request failed for {}: {error}", request.url))?;
        read_response(response, &request.url)
    }
}

/// Registry HTTP client configuration.
pub struct RegistryHttpClient {
    base_url: String,
    bearer_token: Option<String>,
    transport: Box<dyn Transport>,
}

impl std::fmt::Debug for RegistryHttpClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RegistryHttpClient")
            .field("base_url", &self.base_url)
            .field("authenticated", &self.bearer_token.is_some())
            .finish_non_exhaustive()
    }
}

impl RegistryHttpClient {
    /// Construct a client for an HTTP(S) registry origin.
    ///
    /// # Errors
    /// Returns an error when the URL is not a valid HTTP(S) origin, when
    /// credentials are supplied over plain HTTP, or when the bearer token is
    /// empty.
    pub fn new(base_url: &str, bearer_token: Option<&str>) -> Result<Self, String> {
        Self::with_transport(base_url, bearer_token, Box::new(UreqTransport))
    }

    fn with_transport(
        base_url: &str,
        bearer_token: Option<&str>,
        transport: Box<dyn Transport>,
    ) -> Result<Self, String> {
        let uri: ureq::http::Uri = base_url
            .parse()
            .map_err(|_| "registry URL must be a valid HTTP(S) origin".to_owned())?;
        let scheme = uri.scheme_str();
        let valid_authority = uri
            .authority()
            .is_some_and(|authority| !authority.as_str().contains('@'));
        if !matches!(scheme, Some("http" | "https"))
            || !valid_authority
            || uri
                .path_and_query()
                .is_some_and(|value| value.as_str() != "/")
            || base_url.ends_with('/')
        {
            return Err(
                "registry URL must be a bare HTTP(S) origin without userinfo, path, query, or trailing slash"
                    .to_owned(),
            );
        }
        if scheme == Some("http") && bearer_token.is_some() {
            return Err("refusing to send registry credentials over insecure HTTP".to_owned());
        }
        if bearer_token.is_some_and(|token| token.trim().is_empty()) {
            return Err("registry bearer token must not be empty".to_owned());
        }
        Ok(Self {
            base_url: base_url.to_owned(),
            bearer_token: bearer_token.map(str::to_owned),
            transport,
        })
    }

    /// Fetch an immutable package archive by exact package identity.
    ///
    /// # Errors
    /// Returns an error when the package identity is invalid or the HTTP request
    /// fails.
    pub fn fetch_package(&self, name: &str, version: &str) -> Result<Vec<u8>, String> {
        self.execute(Method::Get, &package_path(name, version)?, Vec::new())
    }

    /// Publish an immutable package archive by exact package identity.
    ///
    /// # Errors
    /// Returns an error when the package identity is invalid or the HTTP request
    /// fails.
    pub fn publish_package(
        &self,
        name: &str,
        version: &str,
        archive: Vec<u8>,
    ) -> Result<(), String> {
        self.execute(Method::Put, &package_path(name, version)?, archive)?;
        Ok(())
    }

    fn execute(&self, method: Method, path: &str, body: Vec<u8>) -> Result<Vec<u8>, String> {
        self.transport.execute(Request {
            method,
            url: format!("{}{}", self.base_url, path),
            authorization: self
                .bearer_token
                .as_ref()
                .map(|token| format!("Bearer {token}")),
            body,
        })
    }
}

fn package_path(name: &str, version: &str) -> Result<String, String> {
    crate::commands::validate_package_identity(name, version)
        .map_err(|diagnostics| diagnostics.join("; "))?;
    Ok(format!("/v1/packages/{name}/{version}/archive"))
}

fn read_response(
    mut response: ureq::http::Response<ureq::Body>,
    url: &str,
) -> Result<Vec<u8>, String> {
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

#[cfg(test)]
#[path = "registry_http_test.rs"]
mod tests;
