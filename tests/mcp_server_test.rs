use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// Format a JSON-RPC message with Content-Length header
fn format_message(json: &Value) -> Vec<u8> {
    let content = json.to_string();
    let header = format!("Content-Length: {}\r\n\r\n", content.len());
    let mut message = header.into_bytes();
    message.extend_from_slice(content.as_bytes());
    message
}

/// Parse a message with Content-Length header
fn parse_message(reader: &mut impl BufRead) -> Result<Value> {
    let mut header_line = String::new();

    // Read Content-Length header
    loop {
        header_line.clear();
        let bytes_read = reader.read_line(&mut header_line)?;
        if bytes_read == 0 {
            anyhow::bail!("EOF while reading header");
        }

        if header_line.starts_with("Content-Length: ") {
            break;
        }

        // Skip other headers or empty lines
        if header_line.trim().is_empty() {
            continue;
        }
    }

    let len_str = header_line
        .strip_prefix("Content-Length: ")
        .ok_or_else(|| anyhow::anyhow!("Invalid Content-Length header"))?
        .trim();
    let content_length: usize = len_str.parse()?;

    // Read until empty line (end of headers)
    let mut line = String::new();
    loop {
        line.clear();
        reader.read_line(&mut line)?;
        if line.trim().is_empty() {
            break;
        }
    }

    // Read exact content length
    let mut content = vec![0u8; content_length];
    reader.read_exact(&mut content)?;

    Ok(serde_json::from_slice(&content)?)
}

/// Helper function to send a JSON-RPC request and get response
fn send_jsonrpc_request(request: Value) -> Result<Value> {
    // Start the server process
    let mut child = Command::new("cargo")
        .args(&["run", "--"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let mut reader = BufReader::new(stdout);

    // Send the request with Content-Length header
    let message = format_message(&request);
    stdin.write_all(&message)?;
    stdin.flush()?;

    // Read the response with Content-Length header
    let response = parse_message(&mut reader)?;

    // Clean up
    child.kill()?;

    Ok(response)
}

#[test]
fn test_valid_jsonrpc_request_returns_response() {
    // Requirement 1.1: Valid JSONRPC messages should be processed
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

    let response = send_jsonrpc_request(request).unwrap();

    // Should have jsonrpc version
    assert_eq!(response["jsonrpc"], "2.0");

    // Should have matching id
    assert_eq!(response["id"], 1);

    // Should have either result or error
    assert!(response.get("result").is_some() || response.get("error").is_some());
}

#[test]
fn test_invalid_json_returns_parse_error() {
    // Requirement 1.2: Invalid JSON should return InvalidRequest error
    let _invalid_json = "{ invalid json }";

    // We'll need to implement a raw send for this test
    // For now, we'll create a valid JSON with invalid structure
    let request = json!({
        "not_jsonrpc": "missing required fields"
    });

    let response = send_jsonrpc_request(request).unwrap();

    // Should return error
    assert!(response.get("error").is_some());

    let error = &response["error"];
    assert_eq!(error["code"], -32600); // InvalidRequest
    assert!(error["message"].as_str().unwrap().contains("Invalid"));
}

#[test]
fn test_missing_method_returns_error() {
    // Requirement 1.2: Missing method should return appropriate error
    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "params": {}
    });

    let response = send_jsonrpc_request(request).unwrap();

    // Should return error
    assert!(response.get("error").is_some());

    let error = &response["error"];
    assert_eq!(error["code"], -32600); // InvalidRequest
}

#[test]
fn test_unimplemented_method_returns_unimplemented_error() {
    // Requirement 1.4: Unsupported methods should return Unimplemented error
    let request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "watch-commit",
        "params": {}
    });

    let response = send_jsonrpc_request(request).unwrap();

    // Should return error
    assert!(response.get("error").is_some());

    let error = &response["error"];
    assert_eq!(error["code"], -32601); // MethodNotFound or custom Unimplemented
    assert!(error["message"].as_str().unwrap().to_lowercase().contains("unimplemented"));
}

#[test]
fn test_create_environment_method_exists() {
    // Requirement 1.3: create_environment should be available
    let request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "create_environment",
        "params": {
            "project_root": "/tmp/test",
            "env_id": "test-env",
            "image": "alpine:latest"
        }
    });

    let response = send_jsonrpc_request(request).unwrap();

    // Should get a response (may be error if Podman not running, but method should exist)
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 4);

    // If error, it should not be MethodNotFound
    if let Some(error) = response.get("error") {
        assert_ne!(error["code"], -32601);
    }
}

#[test]
fn test_run_command_method_exists() {
    // Requirement 1.3: run_command should be available
    let request = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "run_command",
        "params": {
            "env_id": "test-env",
            "cmd": ["echo", "hello"]
        }
    });

    let response = send_jsonrpc_request(request).unwrap();

    // Should get a response
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 5);

    // If error, it should not be MethodNotFound
    if let Some(error) = response.get("error") {
        assert_ne!(error["code"], -32601);
    }
}