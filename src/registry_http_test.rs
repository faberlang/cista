use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn rejects_credentials_over_plain_http() {
    let error = RegistryHttpClient::new("http://cista.dev", Some("secret"))
        .expect_err("plain HTTP credentials must fail closed");
    assert!(error.contains("insecure HTTP"));
}

#[test]
fn sends_anonymous_get_and_reads_success_body() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test registry");
    let address = listener.local_addr().expect("registry address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0; 1024];
        let count = stream.read(&mut request).expect("read request");
        let request = String::from_utf8_lossy(&request[..count]);
        assert!(request.starts_with("GET /v1/packages/tool/1.2.3 HTTP/1.1"));
        assert!(!request.contains("Authorization:"));
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\n\r\npayload")
            .expect("write response");
    });
    let client = RegistryHttpClient::new(&format!("http://{address}"), None)
        .expect("anonymous local client");
    assert_eq!(client.get("/v1/packages/tool/1.2.3").unwrap(), b"payload");
    server.join().expect("server thread");
}

#[test]
fn rejects_non_success_response() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test registry");
    let address = listener.local_addr().expect("registry address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0; 512];
        let _request_len = stream.read(&mut request).expect("read request");
        stream
            .write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n")
            .expect("write response");
    });
    let client = RegistryHttpClient::new(&format!("http://{address}"), None).unwrap();
    assert!(client.get("/v1/private").unwrap_err().contains("401"));
    server.join().expect("server thread");
}
