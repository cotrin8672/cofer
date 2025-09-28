use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// Format a JSON-RPC message with Content-Length header
fn format_message(json: &Value) -> String {
    let content = json.to_string();
    format!("Content-Length: {}\r\n\r\n{}", content.len(), content)
}

/// Parse a message with Content-Length header
fn parse_message(reader: &mut impl BufRead) -> Result<Value> {
    let mut header_line = String::new();

    // Read Content-Length header
    reader.read_line(&mut header_line)?;

    if !header_line.starts_with("Content-Length: ") {
        anyhow::bail!("Missing Content-Length header");
    }

    let len_str = header_line
        .strip_prefix("Content-Length: ")
        .unwrap()
        .trim();
    let content_length: usize = len_str.parse()?;

    // Read empty line after header
    let mut empty = String::new();
    reader.read_line(&mut empty)?;
    if empty.trim() != "" {
        anyhow::bail!("Expected empty line after header");
    }

    // Read exact content length
    let mut content = vec![0u8; content_length];
    reader.read_exact(&mut content)?;

    Ok(serde_json::from_slice(&content)?)
}

#[test]
fn test_format_message() {
    let json = json!({"jsonrpc": "2.0", "id": 1, "method": "test"});
    let formatted = format_message(&json);

    assert!(formatted.starts_with("Content-Length: "));
    assert!(formatted.contains("\r\n\r\n"));
    assert!(formatted.ends_with("}"));
}

#[test]
fn test_parse_message() {
    let json = json!({"jsonrpc": "2.0", "id": 1, "method": "test"});
    let formatted = format_message(&json);
    let mut reader = BufReader::new(formatted.as_bytes());

    let parsed = parse_message(&mut reader).unwrap();
    assert_eq!(parsed, json);
}

#[test]
fn test_mcp_server_with_content_length() {
    // Start server
    let mut child = Command::new("cargo")
        .args(&["run", "--"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start server");

    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    // Send initialize request with Content-Length header
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "0.1.0",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "0.1.0"
            }
        }
    });

    let message = format_message(&request);
    stdin.write_all(message.as_bytes()).expect("Failed to write");
    stdin.flush().expect("Failed to flush");

    // Read response with Content-Length header
    let response = parse_message(&mut reader).expect("Failed to parse response");

    // Verify response structure
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response.get("result").is_some() || response.get("error").is_some());

    // If successful, should have server info
    if let Some(result) = response.get("result") {
        assert!(result.get("serverInfo").is_some());
        assert!(result.get("capabilities").is_some());
    }

    // Clean up
    child.kill().expect("Failed to kill process");
}

#[test]
fn test_mcp_server_invalid_request_with_headers() {
    let mut child = Command::new("cargo")
        .args(&["run", "--"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start server");

    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    // Send invalid JSON
    let request = json!({
        "not_jsonrpc": "invalid"
    });

    let message = format_message(&request);
    stdin.write_all(message.as_bytes()).expect("Failed to write");
    stdin.flush().expect("Failed to flush");

    // Read error response
    let response = parse_message(&mut reader).expect("Failed to parse response");

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("error").is_some());
    assert_eq!(response["error"]["code"], -32600);

    child.kill().expect("Failed to kill process");
}

#[test]
fn test_mcp_server_method_not_found_with_headers() {
    let mut child = Command::new("cargo")
        .args(&["run", "--"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start server");

    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    // Send unknown method
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "unknown_method",
        "params": {}
    });

    let message = format_message(&request);
    stdin.write_all(message.as_bytes()).expect("Failed to write");
    stdin.flush().expect("Failed to flush");

    // Read error response
    let response = parse_message(&mut reader).expect("Failed to parse response");

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 2);
    assert!(response.get("error").is_some());
    assert_eq!(response["error"]["code"], -32601);

    child.kill().expect("Failed to kill process");
}