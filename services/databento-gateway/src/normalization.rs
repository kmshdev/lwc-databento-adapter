use std::fmt;

use serde_json::json;

use crate::error::GatewayError;
use crate::protocol::{is_safe_integer, SourceSchema};

pub const UNDEF_INT64: i64 = i64::MIN;
pub const PRICE_SCALE: f64 = 1_000_000_000.0;

#[derive(Debug, Clone)]
pub struct RawBaseBar {
    pub dataset: &'static str,
    pub instrument_id: i64,
    pub schema: SourceSchema,
    pub start_ns: i64,
    pub open: i64,
    pub high: i64,
    pub low: i64,
    pub close: i64,
    pub volume: i64,
}

#[derive(Debug, Clone)]
pub struct NormalizedBaseBar {
    pub time: i64,
    pub dataset: String,
    pub instrument_id: i64,
    pub schema: SourceSchema,
    pub open: i64,
    pub high: i64,
    pub low: i64,
    pub close: i64,
    pub volume: i64,
    pub synthetic: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum NormalizationError {
    UndefinedTimestamp,
    UndefinedPrice,
    NonFinitePrice,
    InvalidOhlc,
    VolumeNegative,
    SchemaMismatch,
}

impl fmt::Display for NormalizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UndefinedTimestamp => write!(f, "undefined timestamp"),
            Self::UndefinedPrice => write!(f, "undefined price"),
            Self::NonFinitePrice => write!(f, "non-finite price"),
            Self::InvalidOhlc => write!(f, "ohlc ordering invalid"),
            Self::VolumeNegative => write!(f, "volume negative"),
            Self::SchemaMismatch => write!(f, "schema mismatch"),
        }
    }
}

impl From<NormalizationError> for GatewayError {
    fn from(error: NormalizationError) -> Self {
        match error {
            NormalizationError::UndefinedTimestamp => {
                GatewayError::invalid_request("undefined timestamp")
            }
            NormalizationError::UndefinedPrice => GatewayError::protocol(
                "undefined price",
                crate::protocol::ProviderErrorCode::Internal,
            ),
            NormalizationError::NonFinitePrice => GatewayError::protocol(
                "non-finite price",
                crate::protocol::ProviderErrorCode::Internal,
            ),
            NormalizationError::InvalidOhlc => {
                GatewayError::invalid_request("invalid OHLC ordering")
            }
            NormalizationError::VolumeNegative => {
                GatewayError::invalid_request("volume must be nonnegative")
            }
            NormalizationError::SchemaMismatch => GatewayError::invalid_request("schema mismatch"),
        }
    }
}

pub fn to_epoch_seconds(start_ns: i64) -> Result<i64, NormalizationError> {
    if start_ns == UNDEF_INT64 {
        return Err(NormalizationError::UndefinedTimestamp);
    }
    if start_ns < 0 {
        return Err(NormalizationError::UndefinedTimestamp);
    }

    let seconds = start_ns / 1_000_000_000;
    if !is_safe_integer(seconds) {
        return Err(NormalizationError::UndefinedTimestamp);
    }
    Ok(seconds)
}

pub fn schema_interval_seconds(schema: &SourceSchema) -> i64 {
    match schema {
        SourceSchema::Ohlcv1s => 1,
        SourceSchema::Ohlcv1m => 60,
        SourceSchema::Ohlcv1h => 3600,
        SourceSchema::Ohlcv1d => 86400,
    }
}

pub fn align_range_start_seconds(seconds: i64, schema: &SourceSchema) -> i64 {
    let interval = schema_interval_seconds(schema);
    let remainder = seconds.rem_euclid(interval);
    if remainder == 0 {
        seconds
    } else {
        seconds + (interval - remainder)
    }
}

fn validate_price(raw: i64) -> Result<i64, NormalizationError> {
    if raw == UNDEF_INT64 {
        return Err(NormalizationError::UndefinedPrice);
    }
    Ok(raw)
}

pub fn price_to_f64(raw: i64) -> f64 {
    raw as f64 / PRICE_SCALE
}

pub fn validate_and_normalize_bar(
    bar: RawBaseBar,
) -> Result<NormalizedBaseBar, NormalizationError> {
    let open = validate_price(bar.open)?;
    let high = validate_price(bar.high)?;
    let low = validate_price(bar.low)?;
    let close = validate_price(bar.close)?;

    if bar.volume < 0 {
        return Err(NormalizationError::VolumeNegative);
    }
    if low > open || low > close || open > high || close > high {
        return Err(NormalizationError::InvalidOhlc);
    }

    let time = to_epoch_seconds(bar.start_ns)?;
    Ok(NormalizedBaseBar {
        time,
        dataset: bar.dataset.to_string(),
        instrument_id: bar.instrument_id,
        schema: bar.schema,
        open,
        high,
        low,
        close,
        volume: bar.volume,
        synthetic: false,
    })
}

pub fn as_volume_metadata(v: i64) -> Result<f64, NormalizationError> {
    if v == UNDEF_INT64 {
        return Err(NormalizationError::UndefinedPrice);
    }
    if v < 0 {
        return Err(NormalizationError::VolumeNegative);
    }
    Ok(v as f64)
}

pub fn missing_schema_error() -> GatewayError {
    GatewayError::invalid_request("unsupported schema")
}

pub fn assert_request_bounds(from: i64, to: i64) -> Result<(), GatewayError> {
    if from < 0 || to < 0 {
        return Err(GatewayError::invalid_range(
            "time range must be non-negative",
        ));
    }
    if from >= to {
        return Err(GatewayError::invalid_range("from must be less than to"));
    }
    // Prevent accidental overflow when serialized into frontend-friendly safe integer.
    if !is_safe_integer(from) {
        return Err(GatewayError::invalid_range(
            "from exceeds browser-safe integer",
        ));
    }
    if !is_safe_integer(to) {
        return Err(GatewayError::invalid_range(
            "to exceeds browser-safe integer",
        ));
    }
    Ok(())
}

pub fn metadata_error(message: &'static str) -> GatewayError {
    GatewayError::protocol(message, crate::protocol::ProviderErrorCode::ProtocolError)
}

pub fn as_json_error(message: &str) -> serde_json::Value {
    json!({ "message": message })
}
