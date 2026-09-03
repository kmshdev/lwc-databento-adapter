use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use thiserror::Error;

use crate::protocol::ProviderErrorCode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayErrorBody {
    pub code: ProviderErrorCode,
    pub message: String,
    pub retryable: bool,
    #[serde(default)]
    pub details: Value,
}

#[derive(Debug, Error, Clone)]
pub enum GatewayError {
    #[error("configuration invalid: {message}")]
    Config {
        message: &'static str,
        details: BTreeMap<String, Value>,
    },
    #[error("invalid request: {message}")]
    InvalidRequest {
        code: ProviderErrorCode,
        message: &'static str,
        details: BTreeMap<String, Value>,
        retryable: bool,
    },
    #[error("{message}")]
    Protocol {
        code: ProviderErrorCode,
        message: &'static str,
        details: BTreeMap<String, Value>,
        retryable: bool,
    },
    #[error("internal: {message}")]
    Internal {
        message: &'static str,
        details: BTreeMap<String, Value>,
    },
}

impl GatewayError {
    pub fn error_body(&self) -> GatewayErrorBody {
        match self {
            GatewayError::Config { .. } => GatewayErrorBody {
                code: ProviderErrorCode::Internal,
                message: "Gateway configuration is invalid".to_string(),
                retryable: false,
                details: Value::Object(Default::default()),
            },
            GatewayError::InvalidRequest {
                code,
                message,
                details,
                retryable,
            } => GatewayErrorBody {
                code: *code,
                message: (*message).to_string(),
                retryable: *retryable,
                details: Value::Object(
                    details
                        .iter()
                        .map(|(key, value)| (key.to_string(), value.clone()))
                        .collect(),
                ),
            },
            GatewayError::Protocol {
                code,
                message,
                details,
                retryable,
            } => GatewayErrorBody {
                code: *code,
                message: (*message).to_string(),
                retryable: *retryable,
                details: Value::Object(
                    details
                        .iter()
                        .map(|(key, value)| (key.to_string(), value.clone()))
                        .collect(),
                ),
            },
            GatewayError::Internal { message, details } => GatewayErrorBody {
                code: ProviderErrorCode::Internal,
                message: (*message).to_string(),
                retryable: false,
                details: Value::Object(
                    details
                        .iter()
                        .map(|(key, value)| (key.to_string(), value.clone()))
                        .collect(),
                ),
            },
        }
    }

    pub fn http_status(&self) -> StatusCode {
        match self {
            GatewayError::InvalidRequest { .. } | GatewayError::Protocol { .. } => {
                StatusCode::BAD_REQUEST
            }
            GatewayError::Config { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            GatewayError::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl GatewayError {
    pub fn invalid_request(message: &'static str) -> Self {
        Self::InvalidRequest {
            code: ProviderErrorCode::InvalidRequest,
            message,
            details: BTreeMap::new(),
            retryable: false,
        }
    }

    pub fn invalid_range(message: &'static str) -> Self {
        Self::InvalidRequest {
            code: ProviderErrorCode::InvalidRange,
            message,
            details: BTreeMap::new(),
            retryable: false,
        }
    }

    pub fn range_too_large(message: &'static str) -> Self {
        Self::InvalidRequest {
            code: ProviderErrorCode::RangeTooLarge,
            message,
            details: BTreeMap::new(),
            retryable: false,
        }
    }

    pub fn unsupported_parent_series(message: &'static str) -> Self {
        Self::InvalidRequest {
            code: ProviderErrorCode::UnsupportedParentSeries,
            message,
            details: BTreeMap::new(),
            retryable: false,
        }
    }

    pub fn resolved_instrument_changed(message: &'static str) -> Self {
        Self::Protocol {
            code: ProviderErrorCode::ResolvedInstrumentChanged,
            message,
            details: BTreeMap::new(),
            retryable: false,
        }
    }

    pub fn protocol(message: &'static str, code: ProviderErrorCode) -> Self {
        Self::Protocol {
            code,
            message,
            details: BTreeMap::new(),
            retryable: false,
        }
    }
}
