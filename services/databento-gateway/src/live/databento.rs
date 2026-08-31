//! Opt-in boundary around Databento's stateful Live client.
//!
//! This module deliberately has no environment lookup and is not wired into the
//! WebSocket transport. A composition root must explicitly construct a
//! `LiveClient`, then attach it to a dataset-scoped session. That keeps the API
//! key and network authority on the server side and makes local-beta tests
//! deterministic.

use std::{collections::HashMap, num::NonZeroUsize};

use databento::{
    dbn::{OhlcvMsg, SType, Schema, SymbolMappingMsg, SystemCode, SystemMsg},
    live::{SlowReaderBehavior, Subscription},
    LiveClient,
};
use tokio::sync::mpsc;

use crate::{
    live::session::ResolvedStreamKeyLike,
    normalization::{to_epoch_seconds, NormalizationError, NormalizedBaseBar, UNDEF_INT64},
    protocol::{SourceSchema, SymbolType},
};

/// The only reconnection choices permitted by the live boundary. Resolution is
/// checked by the caller before a new client is attached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconnectAction {
    Retry,
    Exhausted,
    ResolvedInstrumentChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectPolicy {
    pub max_attempts: u8,
}

impl ReconnectPolicy {
    pub const fn next(self, completed_attempts: u8, resolution_unchanged: bool) -> ReconnectAction {
        if !resolution_unchanged {
            ReconnectAction::ResolvedInstrumentChanged
        } else if completed_attempts < self.max_attempts {
            ReconnectAction::Retry
        } else {
            ReconnectAction::Exhausted
        }
    }
}

/// An opt-in request for one dataset-scoped Databento Live session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveSubscriptionRequest {
    pub dataset: String,
    pub symbol: String,
    pub stype_in: SymbolType,
    pub schema: SourceSchema,
    /// Inclusive replay start in Unix seconds.
    pub replay_start_seconds: i64,
    pub output_capacity: NonZeroUsize,
    pub reconnect: ReconnectPolicy,
}

#[derive(Debug, Clone)]
pub enum LiveEvent {
    Bar(NormalizedBaseBar),
    SymbolMapping {
        requested_symbol: String,
        resolved_symbol: String,
        instrument_id: i64,
        effective_from: i64,
    },
    ReplayCompleted,
    Heartbeat,
    Ended,
    Failure(LiveBoundaryError),
}

/// Commands accepted by the one physical Databento client for a dataset.
/// `LiveClient::subscribe` is explicitly supported after `start` by the
/// pinned client, so adding a canonical request does not create another
/// connection. The actor owns the client and is the sole caller of
/// `next_record`, avoiding competing readers.
#[derive(Debug, Clone)]
pub enum DatasetLiveCommand {
    Add {
        key: ResolvedStreamKeyLike,
        request: LiveSubscriptionRequest,
    },
    Close,
}

#[derive(Debug, Clone)]
pub struct DatasetLiveEvent {
    pub key: ResolvedStreamKeyLike,
    pub event: LiveEvent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveBoundaryError {
    SlowConsumer,
    ResolvedInstrumentChanged,
    Databento(String),
    Normalize(NormalizationError),
}

impl From<databento::Error> for LiveBoundaryError {
    fn from(error: databento::Error) -> Self {
        // Databento errors can contain transport context. Do not surface them to
        // browser clients; transport maps this stable category to a typed error.
        Self::Databento(error.to_string())
    }
}

impl From<NormalizationError> for LiveBoundaryError {
    fn from(error: NormalizationError) -> Self {
        Self::Normalize(error)
    }
}

/// Converts DBN OHLCV without ever converting price integers to floating point.
pub fn normalize_dbn_ohlcv(
    dataset: &str,
    schema: SourceSchema,
    bar: &OhlcvMsg,
) -> Result<NormalizedBaseBar, NormalizationError> {
    normalize_dbn_fields(
        dataset,
        schema,
        bar.hd.instrument_id,
        bar.hd.ts_event,
        bar.open,
        bar.high,
        bar.low,
        bar.close,
        bar.volume,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn normalize_dbn_fields(
    dataset: &str,
    schema: SourceSchema,
    instrument_id: u32,
    start_ns: u64,
    open: i64,
    high: i64,
    low: i64,
    close: i64,
    volume: u64,
) -> Result<NormalizedBaseBar, NormalizationError> {
    let start_ns = i64::try_from(start_ns).map_err(|_| NormalizationError::UndefinedTimestamp)?;
    let volume = i64::try_from(volume).map_err(|_| NormalizationError::VolumeNegative)?;
    if [open, high, low, close].contains(&UNDEF_INT64) {
        return Err(NormalizationError::UndefinedPrice);
    }
    if low > open || low > close || open > high || close > high {
        return Err(NormalizationError::InvalidOhlc);
    }
    if volume < 0 {
        return Err(NormalizationError::VolumeNegative);
    }
    Ok(NormalizedBaseBar {
        time: to_epoch_seconds(start_ns)?,
        dataset: dataset.to_string(),
        instrument_id: i64::from(instrument_id),
        schema,
        open,
        high,
        low,
        close,
        volume,
        synthetic: false,
    })
}

/// A dataset-scoped client holder. It never opens a network connection itself;
/// construction of `LiveClient` is the explicit caller opt-in.
pub struct DatabentoLiveSession {
    request: LiveSubscriptionRequest,
    client: Option<LiveClient>,
    references: usize,
}

impl DatabentoLiveSession {
    /// Opens a live client only after the composition root explicitly opted in
    /// by supplying a server-side key.  This function does not log the key.
    pub async fn connect(
        request: LiveSubscriptionRequest,
        api_key: String,
    ) -> Result<Self, LiveBoundaryError> {
        let client = configured_client_future(api_key, request.dataset.clone())?.await?;
        Ok(Self::new(request, client))
    }

    pub fn new(request: LiveSubscriptionRequest, client: LiveClient) -> Self {
        Self {
            request,
            client: Some(client),
            references: 0,
        }
    }

    pub fn dataset(&self) -> &str {
        &self.request.dataset
    }

    pub fn acquire(&mut self) -> usize {
        self.references += 1;
        self.references
    }

    pub fn output_channel(&self) -> (mpsc::Sender<LiveEvent>, mpsc::Receiver<LiveEvent>) {
        mpsc::channel(self.request.output_capacity.get())
    }

    /// Returns whether the final reference was released. Callers must not make
    /// an upstream unsubscribe: Databento subscriptions end with client close.
    pub async fn release(&mut self) -> Result<bool, LiveBoundaryError> {
        self.references = self.references.saturating_sub(1);
        if self.references != 0 {
            return Ok(false);
        }
        if let Some(client) = self.client.as_mut() {
            client.close().await?;
        }
        self.client.take();
        Ok(true)
    }

    pub fn subscription(&self) -> Subscription {
        Subscription::builder()
            .symbols(self.request.symbol.clone())
            .schema(source_schema_to_dbn(self.request.schema))
            .stype_in(symbol_type_to_dbn(self.request.stype_in))
            .start(
                time::OffsetDateTime::UNIX_EPOCH
                    .saturating_add(time::Duration::seconds(self.request.replay_start_seconds)),
            )
            .build()
    }

    /// Runs exactly one Live subscription. It waits for `ReplayCompleted`, then
    /// keeps reading live records until Databento closes the connection. `try_send`
    /// gives the caller deterministic bounded backpressure.
    pub async fn run(&mut self, sender: &mpsc::Sender<LiveEvent>) -> Result<(), LiveBoundaryError> {
        let request = self.request.clone();
        let subscription = self.subscription();
        let client = self
            .client
            .as_mut()
            .ok_or_else(|| LiveBoundaryError::Databento("live client is closed".to_string()))?;
        client.subscribe(subscription).await?;
        client.start().await?;

        loop {
            let Some(record) = client.next_record().await? else {
                send_bounded(sender, LiveEvent::Ended)?;
                return Ok(());
            };
            if let Some(system) = record.get::<SystemMsg>() {
                match system.code {
                    code if code == SystemCode::ReplayCompleted as u8 => {
                        send_bounded(sender, LiveEvent::ReplayCompleted)?;
                    }
                    code if code == SystemCode::Heartbeat as u8 => {
                        send_bounded(sender, LiveEvent::Heartbeat)?;
                    }
                    _ => {}
                }
                continue;
            }
            if let Some(bar) = record.get::<OhlcvMsg>() {
                let normalized = normalize_dbn_ohlcv(&request.dataset, request.schema, bar)?;
                send_bounded(sender, LiveEvent::Bar(normalized))?;
            }
        }
    }
}

/// Owns exactly one physical Live client for a dataset and dynamically adds
/// canonical subscriptions to it. The transport fans these normalized events
/// out through `DatasetLiveRegistry`; this actor never knows browser IDs.
pub struct DatabentoDatasetActor {
    dataset: String,
    client: LiveClient,
    active: HashMap<i64, Vec<ActiveCanonical>>,
    started: bool,
}

#[derive(Debug, Clone)]
struct ActiveCanonical {
    key: ResolvedStreamKeyLike,
    schema: SourceSchema,
}

fn register_canonical(
    active: &mut HashMap<i64, Vec<ActiveCanonical>>,
    canonical: ActiveCanonical,
) -> bool {
    let entries = active.entry(canonical.key.instrument_id).or_default();
    if entries.iter().any(|entry| entry.key == canonical.key) {
        return false;
    }
    entries.push(canonical);
    true
}

fn mapping_event(
    canonical: &ActiveCanonical,
    mapping: &SymbolMappingMsg,
) -> Result<Option<LiveEvent>, LiveBoundaryError> {
    let stype_in = mapping
        .stype_in()
        .map_err(|error| LiveBoundaryError::Databento(error.to_string()))?;
    let Some(stype_in) = dbn_symbol_type(stype_in) else {
        return Ok(None);
    };
    let requested_symbol = mapping
        .stype_in_symbol()
        .map_err(|error| LiveBoundaryError::Databento(error.to_string()))?;
    let matches_request = canonical.key.stype_in == stype_in
        && match stype_in {
            SymbolType::InstrumentId => {
                canonical.key.instrument_id == i64::from(mapping.hd.instrument_id)
            }
            _ => canonical.key.requested_symbol == requested_symbol,
        };
    if !matches_request {
        return Ok(None);
    }
    let resolved_symbol = mapping
        .stype_out_symbol()
        .map_err(|error| LiveBoundaryError::Databento(error.to_string()))?;
    let instrument_id = i64::from(mapping.hd.instrument_id);
    if canonical.key.instrument_id != instrument_id {
        return Ok(Some(LiveEvent::Failure(
            LiveBoundaryError::ResolvedInstrumentChanged,
        )));
    }
    let start_ns = i64::try_from(mapping.start_ts)
        .or_else(|_| i64::try_from(mapping.hd.ts_event))
        .map_err(|_| LiveBoundaryError::Normalize(NormalizationError::UndefinedTimestamp))?;
    Ok(Some(LiveEvent::SymbolMapping {
        requested_symbol: canonical.key.requested_symbol.clone(),
        resolved_symbol: resolved_symbol.to_string(),
        instrument_id,
        effective_from: to_epoch_seconds(start_ns)?,
    }))
}

impl DatabentoDatasetActor {
    pub async fn connect(dataset: String, api_key: String) -> Result<Self, LiveBoundaryError> {
        let client = configured_client_future(api_key, dataset.clone())?.await?;
        Ok(Self {
            dataset,
            client,
            active: HashMap::new(),
            started: false,
        })
    }

    pub async fn run(
        mut self,
        mut commands: mpsc::Receiver<DatasetLiveCommand>,
        output: mpsc::Sender<DatasetLiveEvent>,
    ) -> Result<(), LiveBoundaryError> {
        loop {
            tokio::select! {
                command = commands.recv() => match command {
                    Some(DatasetLiveCommand::Add { key, request }) => {
                        if request.dataset != self.dataset {
                            return Err(LiveBoundaryError::Databento("dataset actor request mismatch".to_string()));
                        }
                        if !register_canonical(&mut self.active, ActiveCanonical {
                            key,
                            schema: request.schema,
                        }) {
                            continue;
                        }
                        self.client.subscribe(subscription_for(&request)).await?;
                        if !self.started {
                            self.client.start().await?;
                            self.started = true;
                        }
                    }
                    Some(DatasetLiveCommand::Close) | None => {
                        self.client.close().await?;
                        return Ok(());
                    }
                },
                record = self.client.next_record(), if self.started => {
                    let Some(record) = record? else {
                        self.broadcast(&output, LiveEvent::Ended)?;
                        return Ok(());
                    };
                    if let Some(system) = record.get::<SystemMsg>() {
                        match system.code {
                            code if code == SystemCode::ReplayCompleted as u8 => self.broadcast(&output, LiveEvent::ReplayCompleted)?,
                            code if code == SystemCode::Heartbeat as u8 => self.broadcast(&output, LiveEvent::Heartbeat)?,
                            _ => {}
                        }
                        continue;
                    }
                    if let Some(mapping) = record.get::<SymbolMappingMsg>() {
                        let active = self.active.values().flatten().cloned().collect::<Vec<_>>();
                        for canonical in active {
                            if let Some(event) = mapping_event(&canonical, mapping)? {
                                send_dataset_bounded(&output, DatasetLiveEvent {
                                    key: canonical.key,
                                    event,
                                })?;
                            }
                        }
                        continue;
                    }
                    if let Some(bar) = record.get::<OhlcvMsg>() {
                        let instrument_id = i64::from(bar.hd.instrument_id);
                        let active = self.active.get(&instrument_id).cloned().unwrap_or_default();
                        for canonical in active {
                            let normalized = normalize_dbn_ohlcv(&self.dataset, canonical.schema, bar)?;
                            send_dataset_bounded(&output, DatasetLiveEvent { key: canonical.key, event: LiveEvent::Bar(normalized) })?;
                        }
                    }
                }
            }
        }
    }

    fn broadcast(
        &self,
        output: &mpsc::Sender<DatasetLiveEvent>,
        event: LiveEvent,
    ) -> Result<(), LiveBoundaryError> {
        for canonical in self.active.values().flatten() {
            send_dataset_bounded(
                output,
                DatasetLiveEvent {
                    key: canonical.key.clone(),
                    event: event.clone(),
                },
            )?;
        }
        Ok(())
    }
}

fn subscription_for(request: &LiveSubscriptionRequest) -> Subscription {
    Subscription::builder()
        .symbols(request.symbol.clone())
        .schema(source_schema_to_dbn(request.schema))
        .stype_in(symbol_type_to_dbn(request.stype_in))
        .start(
            time::OffsetDateTime::UNIX_EPOCH
                .saturating_add(time::Duration::seconds(request.replay_start_seconds)),
        )
        .build()
}

fn send_dataset_bounded(
    sender: &mpsc::Sender<DatasetLiveEvent>,
    event: DatasetLiveEvent,
) -> Result<(), LiveBoundaryError> {
    sender
        .try_send(event)
        .map_err(|_| LiveBoundaryError::SlowConsumer)
}

fn send_bounded(
    sender: &mpsc::Sender<LiveEvent>,
    event: LiveEvent,
) -> Result<(), LiveBoundaryError> {
    sender
        .try_send(event)
        .map_err(|_| LiveBoundaryError::SlowConsumer)
}

fn source_schema_to_dbn(schema: SourceSchema) -> Schema {
    match schema {
        SourceSchema::Ohlcv1s => Schema::Ohlcv1S,
        SourceSchema::Ohlcv1m => Schema::Ohlcv1M,
        SourceSchema::Ohlcv1h => Schema::Ohlcv1H,
        SourceSchema::Ohlcv1d => Schema::Ohlcv1D,
    }
}

fn symbol_type_to_dbn(stype: SymbolType) -> databento::dbn::SType {
    match stype {
        SymbolType::RawSymbol => databento::dbn::SType::RawSymbol,
        SymbolType::InstrumentId => databento::dbn::SType::InstrumentId,
        SymbolType::Parent => databento::dbn::SType::Parent,
        SymbolType::Continuous => databento::dbn::SType::Continuous,
    }
}

fn dbn_symbol_type(stype: SType) -> Option<SymbolType> {
    match stype {
        SType::RawSymbol => Some(SymbolType::RawSymbol),
        SType::InstrumentId => Some(SymbolType::InstrumentId),
        SType::Parent => Some(SymbolType::Parent),
        SType::Continuous => Some(SymbolType::Continuous),
        _ => None,
    }
}

/// Compile-only proof of the pinned Live builder without making a connection.
pub fn configured_client_future(
    key: String,
    dataset: String,
) -> databento::Result<impl std::future::Future<Output = databento::Result<LiveClient>>> {
    Ok(LiveClient::builder()
        .key(key)?
        .dataset(dataset)
        .slow_reader_behavior(SlowReaderBehavior::Warn)
        .build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_integer_dbn_fields_without_float_conversion() {
        let bar = normalize_dbn_fields(
            "GLBX.MDP3",
            SourceSchema::Ohlcv1m,
            42,
            1_700_000_000_000_000_000,
            5_000_000_000,
            6_000_000_000,
            4_000_000_000,
            5_500_000_000,
            7,
        )
        .unwrap();
        assert_eq!(bar.time, 1_700_000_000);
        assert_eq!(bar.open, 5_000_000_000);
        assert_eq!(bar.volume, 7);
    }

    #[test]
    fn rejects_dbn_values_that_cannot_fit_gateway_integers() {
        assert_eq!(
            normalize_dbn_fields(
                "GLBX.MDP3",
                SourceSchema::Ohlcv1m,
                42,
                u64::MAX,
                1,
                1,
                1,
                1,
                1,
            )
            .unwrap_err(),
            NormalizationError::UndefinedTimestamp
        );
    }

    #[test]
    fn reconnect_policy_fails_closed_when_resolution_changes() {
        let policy = ReconnectPolicy { max_attempts: 2 };
        assert_eq!(
            policy.next(0, false),
            ReconnectAction::ResolvedInstrumentChanged
        );
        assert_eq!(policy.next(0, true), ReconnectAction::Retry);
        assert_eq!(policy.next(2, true), ReconnectAction::Exhausted);
    }

    #[test]
    fn symbol_mapping_is_applied_before_live_bar_routing() {
        let canonical = ActiveCanonical {
            key: ResolvedStreamKeyLike {
                dataset: "GLBX.MDP3".to_string(),
                requested_symbol: "ES.c.0".to_string(),
                stype_in: SymbolType::Continuous,
                resolution: crate::protocol::Resolution::OneMinute,
                gap_policy: crate::protocol::GapPolicy::PreserveGaps,
                resolved_symbol: "42".to_string(),
                instrument_id: 42,
                source_schema: SourceSchema::Ohlcv1m,
            },
            schema: SourceSchema::Ohlcv1m,
        };
        let mapping = databento::dbn::SymbolMappingMsg::new(
            42,
            1_700_000_000_000_000_000,
            databento::dbn::SType::Continuous,
            "ES.c.0",
            databento::dbn::SType::RawSymbol,
            "ESZ4",
            u64::MAX,
            u64::MAX,
        )
        .unwrap();

        let event = mapping_event(&canonical, &mapping).unwrap().unwrap();

        assert!(matches!(
            event,
            LiveEvent::SymbolMapping {
                instrument_id: 42,
                effective_from: 1_700_000_000,
                ..
            }
        ));
    }

    #[test]
    fn symbol_mapping_change_fails_the_session_pinned_stream() {
        let canonical = ActiveCanonical {
            key: ResolvedStreamKeyLike {
                dataset: "GLBX.MDP3".to_string(),
                requested_symbol: "ES.c.0".to_string(),
                stype_in: SymbolType::Continuous,
                resolution: crate::protocol::Resolution::OneMinute,
                gap_policy: crate::protocol::GapPolicy::PreserveGaps,
                resolved_symbol: "ESZ4".to_string(),
                instrument_id: 42,
                source_schema: SourceSchema::Ohlcv1m,
            },
            schema: SourceSchema::Ohlcv1m,
        };
        let mapping = databento::dbn::SymbolMappingMsg::new(
            43,
            1_700_000_000_000_000_000,
            databento::dbn::SType::Continuous,
            "ES.c.0",
            databento::dbn::SType::RawSymbol,
            "ESH5",
            1_700_000_000_000_000_000,
            u64::MAX,
        )
        .unwrap();

        assert!(matches!(
            mapping_event(&canonical, &mapping).unwrap(),
            Some(LiveEvent::Failure(
                LiveBoundaryError::ResolvedInstrumentChanged
            ))
        ));
    }

    #[test]
    fn reactivating_a_canonical_stream_does_not_duplicate_routing() {
        let canonical = ActiveCanonical {
            key: ResolvedStreamKeyLike {
                dataset: "GLBX.MDP3".to_string(),
                requested_symbol: "ES.c.0".to_string(),
                stype_in: SymbolType::Continuous,
                resolution: crate::protocol::Resolution::OneMinute,
                gap_policy: crate::protocol::GapPolicy::PreserveGaps,
                resolved_symbol: "42".to_string(),
                instrument_id: 42,
                source_schema: SourceSchema::Ohlcv1m,
            },
            schema: SourceSchema::Ohlcv1m,
        };
        let mut active = HashMap::new();

        assert!(register_canonical(&mut active, canonical.clone()));
        assert!(!register_canonical(&mut active, canonical));
        assert_eq!(active.get(&42).unwrap().len(), 1);
    }

    #[test]
    fn bounded_output_reports_slow_consumer_without_dropping_lifecycle_events() {
        let (sender, _receiver) = mpsc::channel(1);
        send_bounded(&sender, LiveEvent::ReplayCompleted).unwrap();
        assert!(matches!(
            send_bounded(&sender, LiveEvent::Ended),
            Err(LiveBoundaryError::SlowConsumer)
        ));
    }

    #[test]
    fn configured_client_future_typechecks_without_polling_network() {
        std::mem::drop(configured_client_future("0".repeat(32), "GLBX.MDP3".to_string()).unwrap());
    }
}
