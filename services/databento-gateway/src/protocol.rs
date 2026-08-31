use serde::{de::Error as DeError, Deserialize, Serialize};
use serde_json::Error as SerdeError;

use std::fmt;

pub const PROTOCOL_VERSION: u8 = 1;
pub const MAX_SAFE_INTEGER_SECONDS: i64 = 9_007_199_254_740_991;

const fn protocol_version() -> u8 {
    PROTOCOL_VERSION
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolType {
    #[serde(rename = "raw_symbol")]
    RawSymbol,
    #[serde(rename = "instrument_id")]
    InstrumentId,
    #[serde(rename = "parent")]
    Parent,
    #[serde(rename = "continuous")]
    Continuous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Resolution {
    #[serde(rename = "1s")]
    OneSecond,
    #[serde(rename = "5s")]
    FiveSecond,
    #[serde(rename = "15s")]
    FifteenSecond,
    #[serde(rename = "30s")]
    ThirtySecond,
    #[serde(rename = "1m")]
    OneMinute,
    #[serde(rename = "5m")]
    FiveMinute,
    #[serde(rename = "15m")]
    FifteenMinute,
    #[serde(rename = "30m")]
    ThirtyMinute,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "2h")]
    TwoHour,
    #[serde(rename = "4h")]
    FourHour,
    #[serde(rename = "1d")]
    OneDay,
}

impl Resolution {
    pub const ORDERED: [Resolution; 12] = [
        Resolution::OneSecond,
        Resolution::FiveSecond,
        Resolution::FifteenSecond,
        Resolution::ThirtySecond,
        Resolution::OneMinute,
        Resolution::FiveMinute,
        Resolution::FifteenMinute,
        Resolution::ThirtyMinute,
        Resolution::OneHour,
        Resolution::TwoHour,
        Resolution::FourHour,
        Resolution::OneDay,
    ];

    pub const fn as_seconds(self) -> i64 {
        match self {
            Self::OneSecond => 1,
            Self::FiveSecond => 5,
            Self::FifteenSecond => 15,
            Self::ThirtySecond => 30,
            Self::OneMinute => 60,
            Self::FiveMinute => 300,
            Self::FifteenMinute => 900,
            Self::ThirtyMinute => 1800,
            Self::OneHour => 3600,
            Self::TwoHour => 7200,
            Self::FourHour => 14400,
            Self::OneDay => 86400,
        }
    }

    pub const fn source_schema(self) -> SourceSchema {
        match self {
            Self::OneSecond | Self::FiveSecond | Self::FifteenSecond | Self::ThirtySecond => {
                SourceSchema::Ohlcv1s
            }
            Self::OneMinute | Self::FiveMinute | Self::FifteenMinute | Self::ThirtyMinute => {
                SourceSchema::Ohlcv1m
            }
            Self::OneHour | Self::TwoHour | Self::FourHour => SourceSchema::Ohlcv1h,
            Self::OneDay => SourceSchema::Ohlcv1d,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum GapPolicy {
    #[default]
    PreserveGaps,
    Whitespace,
    CarryForward,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderState {
    Idle,
    Connecting,
    Replaying,
    Live,
    Reconnecting,
    Failed,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusReason {
    InitialConnect,
    HandoffReplay,
    ReplayCompleted,
    UpstreamDisconnect,
    DownstreamDisconnect,
    RetryScheduled,
    RetryExhausted,
    ClientUnsubscribe,
    ServerShutdown,
    SlowConsumer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum SourceSchema {
    #[serde(rename = "ohlcv-1s")]
    Ohlcv1s,
    #[serde(rename = "ohlcv-1m")]
    Ohlcv1m,
    #[serde(rename = "ohlcv-1h")]
    Ohlcv1h,
    #[serde(rename = "ohlcv-1d")]
    Ohlcv1d,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderErrorCode {
    InvalidRequest,
    InvalidRange,
    RangeTooLarge,
    OriginForbidden,
    DatasetForbidden,
    UnsupportedDataset,
    UnsupportedSchema,
    UnsupportedResolution,
    SymbolNotFound,
    SymbolMappingFailed,
    UnsupportedParentSeries,
    ResolvedInstrumentChanged,
    UnsupportedLiveSymbology,
    AccessDenied,
    QuotaExceeded,
    SlowConsumer,
    ReplayUnavailable,
    UpstreamUnavailable,
    Cancelled,
    ProtocolError,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderError {
    pub code: ProviderErrorCode,
    pub message: String,
    pub retryable: bool,
    #[serde(default)]
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub v: u8,
    pub request_id: String,
    pub error: ProviderError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BarRequest {
    #[serde(default = "protocol_version")]
    pub v: u8,
    pub dataset: String,
    pub symbol: String,
    pub stype_in: SymbolType,
    pub resolution: Resolution,
    #[serde(default)]
    pub gap_policy: Option<GapPolicy>,
}

impl BarRequest {
    pub fn gap_policy_or_default(&self) -> GapPolicy {
        self.gap_policy.unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryRequest {
    #[serde(default = "protocol_version")]
    pub v: u8,
    pub dataset: String,
    pub symbol: String,
    pub stype_in: SymbolType,
    pub resolution: Resolution,
    pub from: i64,
    pub to: i64,
    #[serde(default)]
    pub gap_policy: Option<GapPolicy>,
}

impl HistoryRequest {
    pub fn gap_policy_or_default(&self) -> GapPolicy {
        self.gap_policy.unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveRequest {
    #[serde(default = "protocol_version")]
    pub v: u8,
    pub dataset: String,
    pub symbols: Vec<String>,
    pub stype_in: SymbolType,
    pub from: i64,
    pub to: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchRequest {
    #[serde(default = "protocol_version")]
    pub v: u8,
    pub dataset: String,
    pub query: String,
    pub stype_in: SymbolType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub dataset: String,
    pub symbol: String,
    pub stype_in: SymbolType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub v: u8,
    pub request_id: String,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolMapping {
    pub dataset: String,
    pub requested_symbol: String,
    pub resolved_symbol: String,
    pub instrument_id: i64,
    pub effective_from: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_to: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveResponse {
    pub v: u8,
    pub request_id: String,
    pub mappings: Vec<SymbolMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublisherMetadata {
    pub publisher_id: i64,
    pub name: String,
    pub venue: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetMetadata {
    pub dataset: String,
    pub schemas: Vec<String>,
    pub publishers: Vec<PublisherMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available_from: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available_to: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetResponse {
    pub v: u8,
    pub request_id: String,
    pub metadata: DatasetMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChartBar {
    Whitespace {
        time: i64,
    },
    Candlestick {
        time: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BarMetadata {
    pub dataset: String,
    pub requested_symbol: String,
    pub resolved_symbol: String,
    pub instrument_id: i64,
    pub source_schema: SourceSchema,
    pub synthetic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataPoint {
    pub time: i64,
    #[serde(flatten)]
    pub meta: BarMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumePoint {
    pub time: i64,
    pub value: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BarPageResponse {
    pub v: u8,
    pub request_id: String,
    pub bars: Vec<ChartBar>,
    pub volumes: Vec<VolumePoint>,
    pub metadata: Vec<MetadataPoint>,
}

impl BarPageResponse {
    pub fn validate_time_range(&self) -> Result<(), ProtocolTimeError> {
        if self
            .bars
            .iter()
            .any(|bar| bar.time() > MAX_SAFE_INTEGER_SECONDS)
        {
            return Err(ProtocolTimeError::UnsafeInteger);
        }
        if self
            .volumes
            .iter()
            .any(|volume| volume.time > MAX_SAFE_INTEGER_SECONDS)
        {
            return Err(ProtocolTimeError::UnsafeInteger);
        }
        if self
            .metadata
            .iter()
            .any(|meta| meta.time > MAX_SAFE_INTEGER_SECONDS)
        {
            return Err(ProtocolTimeError::UnsafeInteger);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RequestEnvelope {
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubscribeBarsCommand {
    pub v: u8,
    pub command_id: String,
    pub subscription_id: String,
    pub request: BarRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenBarsCommand {
    pub v: u8,
    pub command_id: String,
    pub subscription_id: String,
    pub request: HistoryRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResumeBarsCommand {
    pub v: u8,
    pub command_id: String,
    pub subscription_id: String,
    pub resume_from: i64,
    pub request: BarRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnsubscribeCommand {
    pub v: u8,
    pub command_id: String,
    pub subscription_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelCommand {
    pub v: u8,
    pub command_id: String,
    pub target_command_id: String,
    pub subscription_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientCommand {
    #[serde(rename = "subscribe_bars")]
    SubscribeBars(SubscribeBarsCommand),
    #[serde(rename = "open_bars")]
    OpenBars(OpenBarsCommand),
    #[serde(rename = "resume_bars")]
    ResumeBars(ResumeBarsCommand),
    #[serde(rename = "unsubscribe")]
    Unsubscribe(UnsubscribeCommand),
    #[serde(rename = "cancel")]
    Cancel(CancelCommand),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubscribedEvent {
    pub v: u8,
    pub command_id: String,
    pub subscription_id: String,
    pub state: ProviderState,
    pub resolved_symbols: Vec<SymbolMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotEvent {
    pub v: u8,
    pub subscription_id: String,
    pub bars: Vec<ChartBar>,
    pub volumes: Vec<VolumePoint>,
    pub metadata: Vec<MetadataPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BarEvent {
    pub v: u8,
    pub subscription_id: String,
    pub data: ChartBar,
    pub volume: Option<VolumePoint>,
    pub meta: BarMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatusEvent {
    pub v: u8,
    pub subscription_id: String,
    pub state: ProviderState,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<StatusReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SymbolMappingEvent {
    pub v: u8,
    pub subscription_id: String,
    pub requested_symbol: String,
    pub resolved_symbol: String,
    pub instrument_id: i64,
    pub effective_from: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnsubscribedEvent {
    pub v: u8,
    pub command_id: String,
    pub subscription_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelledEvent {
    pub v: u8,
    pub command_id: String,
    pub target_command_id: String,
    pub subscription_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ErrorEvent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subscription_id: Option<String>,
    pub v: u8,
    pub error: ProviderError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HeartbeatEvent {
    pub v: u8,
    pub server_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    #[serde(rename = "subscribed")]
    Subscribed(SubscribedEvent),
    #[serde(rename = "snapshot")]
    Snapshot(SnapshotEvent),
    #[serde(rename = "bar")]
    Bar(BarEvent),
    #[serde(rename = "status")]
    Status(StatusEvent),
    #[serde(rename = "symbol_mapping")]
    SymbolMapping(SymbolMappingEvent),
    #[serde(rename = "unsubscribed")]
    Unsubscribed(UnsubscribedEvent),
    #[serde(rename = "cancelled")]
    Cancelled(CancelledEvent),
    #[serde(rename = "error")]
    Error(ErrorEvent),
    #[serde(rename = "heartbeat")]
    Heartbeat(HeartbeatEvent),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct RequestedStreamKey {
    pub dataset: String,
    pub requested_symbol: String,
    pub stype_in: SymbolType,
    pub resolution: Resolution,
    pub gap_policy: GapPolicy,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ResolvedStreamKey {
    pub request: RequestedStreamKey,
    pub resolved_symbol: String,
    pub instrument_id: i64,
    pub source_schema: SourceSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamDirection {
    Upstream,
    Downstream,
}

#[derive(Debug)]
pub enum ProtocolTimeError {
    UnsafeInteger,
}

impl fmt::Display for ProtocolTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsafeInteger => {
                write!(
                    f,
                    "value exceeds browser-safe integer {MAX_SAFE_INTEGER_SECONDS}"
                )
            }
        }
    }
}

pub fn parse_client_command(value: &[u8]) -> Result<ClientCommand, SerdeError> {
    let command: ClientCommand = serde_json::from_slice(value)?;
    let version = match &command {
        ClientCommand::SubscribeBars(value) => value.v,
        ClientCommand::OpenBars(value) => value.v,
        ClientCommand::ResumeBars(value) => value.v,
        ClientCommand::Unsubscribe(value) => value.v,
        ClientCommand::Cancel(value) => value.v,
    };
    if version != PROTOCOL_VERSION {
        return Err(DeError::custom("unsupported protocol version"));
    }
    Ok(command)
}

pub fn parse_server_event(value: &[u8]) -> Result<ServerEvent, SerdeError> {
    let event: ServerEvent = serde_json::from_slice(value)?;
    validate_server_event(&event)?;
    Ok(event)
}

fn validate_server_event(event: &ServerEvent) -> Result<(), SerdeError> {
    if let ServerEvent::Bar(BarEvent {
        data,
        volume: Some(volume),
        ..
    }) = event
    {
        if bar_time(data) != volume.time {
            return Err(DeError::custom("bar.volume.time must equal bar.data.time"));
        }
    }
    Ok(())
}

fn bar_time(bar: &ChartBar) -> i64 {
    match bar {
        ChartBar::Whitespace { time } => *time,
        ChartBar::Candlestick { time, .. } => *time,
    }
}

impl ChartBar {
    pub const fn time(&self) -> i64 {
        match self {
            Self::Whitespace { time } => *time,
            Self::Candlestick { time, .. } => *time,
        }
    }
}

pub fn is_safe_integer(time: i64) -> bool {
    (0..=MAX_SAFE_INTEGER_SECONDS).contains(&time)
}
