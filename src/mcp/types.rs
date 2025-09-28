use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// JSON-RPC request structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    pub params: Option<Value>,
}

/// JSON-RPC response structure
#[derive(Debug, Clone, Serialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

/// JSON-RPC error structure
#[derive(Debug, Clone, Serialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl McpError {
    /// Invalid Request error (-32600)
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: -32600,
            message: message.into(),
            data: None,
        }
    }

    /// Method Not Found error (-32601)
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self {
            code: -32601,
            message: format!("Method '{}' not found", method.into()),
            data: None,
        }
    }

    /// Invalid Params error (-32602)
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
            data: None,
        }
    }

    /// Internal Error (-32603)
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: message.into(),
            data: None,
        }
    }
}

impl fmt::Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for McpError {}

impl From<anyhow::Error> for McpError {
    fn from(err: anyhow::Error) -> Self {
        Self::internal_error(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mcp_request_deserialization() {
        let json_str = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "test_method",
            "params": {"key": "value"}
        }"#;

        let request: McpRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, Some(json!(1)));
        assert_eq!(request.method, "test_method");
        assert_eq!(request.params, Some(json!({"key": "value"})));
    }

    #[test]
    fn test_mcp_response_serialization() {
        let response = McpResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            result: Some(json!({"status": "ok"})),
            error: None,
        };

        let json_str = serde_json::to_string(&response).unwrap();
        assert!(json_str.contains("\"jsonrpc\":\"2.0\""));
        assert!(json_str.contains("\"id\":1"));
        assert!(json_str.contains("\"result\""));
        assert!(!json_str.contains("\"error\""));
    }

    #[test]
    fn test_mcp_error_serialization() {
        let response = McpResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            result: None,
            error: Some(McpError::invalid_request("Missing required field")),
        };

        let json_str = serde_json::to_string(&response).unwrap();
        assert!(json_str.contains("\"error\""));
        assert!(json_str.contains("-32600"));
        assert!(!json_str.contains("\"result\""));
    }

    #[test]
    fn test_error_constructors() {
        let err = McpError::invalid_request("test");
        assert_eq!(err.code, -32600);

        let err = McpError::method_not_found("unknown");
        assert_eq!(err.code, -32601);
        assert!(err.message.contains("unknown"));

        let err = McpError::invalid_params("bad params");
        assert_eq!(err.code, -32602);

        let err = McpError::internal_error("internal");
        assert_eq!(err.code, -32603);
    }
}