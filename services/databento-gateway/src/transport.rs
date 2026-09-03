use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::http::{header::SEC_WEBSOCKET_PROTOCOL, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot};

use crate::aggregation::{aggregate, AggregatedBar, AggregationConfig};
use crate::config::GatewayConfig;
use crate::error::GatewayError;
use crate::historical::HistoricalSource;
use crate::normalization::schema_interval_seconds;
use crate::normalization::{assert_history_interval_cap, assert_request_bounds, price_to_f64};
use crate::protocol::{
    parse_client_command, BarMetadata, CancelledEvent, ChartBar, ClientCommand, ErrorEvent,
    ErrorResponse, HistoryRequest, MetadataPoint, ProviderError, ProviderErrorCode, ProviderState,
    ResolveRequest, SearchRequest, SearchResponse, SnapshotEvent, StatusEvent, SubscribedEvent,
    SymbolMapping, SymbolType, UnsubscribedEvent, VolumePoint,
};

#[cfg(feature = "databento-compat")]
use crate::normalization::NormalizedBaseBar;
#[cfg(feature = "databento-compat")]
use crate::protocol::BarEvent;

#[cfg(feature = "databento-compat")]
use crate::live::databento::{
    DatabentoDatasetActor, DatasetLiveCommand, DatasetLiveEvent, LiveBoundaryError, LiveEvent,
    LiveSubscriptionRequest, ReconnectPolicy,
};
#[cfg(feature = "databento-compat")]
use crate::live::session::{DatasetLiveRegistry, RegistryAcquireError, ResolvedStreamKeyLike};

pub const LIVE_SUBPROTOCOL: &str = "databento-lwc.v1";
type Outbound = mpsc::Sender<Message>;

#[derive(Clone)]
pub struct AppState {
    pub history_source: std::sync::Arc<dyn HistoricalSource + Send + Sync>,
    pub config: GatewayConfig,
    /// Deliberately process-scoped: a browser reconnect creates a new WebSocket but
    /// must be able to resume its stable subscription id.  Explicit unsubscribe or
    /// cancel removes the entry; socket closure alone does not.
    live_sessions: Arc<Mutex<LiveState>>,
    #[cfg(feature = "databento-compat")]
    live_registry: Arc<Mutex<DatasetLiveRegistry<LiveEvent>>>,
    active_clients: Arc<AtomicUsize>,
    /// Present only when the composition root explicitly selected the official
    /// Databento source. It remains server-only and is never serialized.
    #[cfg(feature = "databento-compat")]
    live_api_key: Option<Arc<str>>,
}

impl AppState {
    pub fn new(
        history_source: std::sync::Arc<dyn HistoricalSource + Send + Sync>,
        config: GatewayConfig,
    ) -> Self {
        Self::new_with_live_key(history_source, config, None)
    }

    pub fn new_with_live_key(
        history_source: std::sync::Arc<dyn HistoricalSource + Send + Sync>,
        config: GatewayConfig,
        live_api_key: Option<String>,
    ) -> Self {
        #[cfg(not(feature = "databento-compat"))]
        let _ = live_api_key;
        #[cfg(feature = "databento-compat")]
        let (max_dataset_sessions, canonical_queue_capacity) =
            (config.max_dataset_sessions, config.canonical_queue_capacity);
        Self {
            history_source,
            config,
            live_sessions: Arc::new(Mutex::new(LiveState::default())),
            #[cfg(feature = "databento-compat")]
            live_registry: Arc::new(Mutex::new(DatasetLiveRegistry::new(
                max_dataset_sessions,
                canonical_queue_capacity,
            ))),
            active_clients: Arc::new(AtomicUsize::new(0)),
            #[cfg(feature = "databento-compat")]
            live_api_key: live_api_key.map(Arc::<str>::from),
        }
    }
}

#[derive(Clone)]
struct ActiveSubscription {
    mappings: Vec<SymbolMapping>,
    request: crate::protocol::BarRequest,
    live_task: Option<LiveTaskControl>,
    /// Used to emit a typed error and disconnect this subscription if the
    /// registry ever reports it as a slow consumer (see `evict_slow_consumer`).
    outbound: Outbound,
    #[cfg(feature = "databento-compat")]
    registry_key: Option<ResolvedStreamKeyLike>,
}

impl ActiveSubscription {
    fn matches(&self, mappings: &[SymbolMapping]) -> bool {
        self.mappings
            .first()
            .zip(mappings.first())
            .is_none_or(|(left, right)| {
                left.instrument_id == right.instrument_id
                    && left.resolved_symbol == right.resolved_symbol
            })
    }
}

#[derive(Default)]
struct LiveState {
    subscriptions: HashMap<String, ActiveSubscription>,
    #[cfg(feature = "databento-compat")]
    dataset_actors: HashMap<String, DatasetActorControl>,
}

#[cfg(feature = "databento-compat")]
#[derive(Clone)]
struct DatasetActorControl {
    commands: mpsc::Sender<DatasetLiveCommand>,
}

#[cfg(feature = "databento-compat")]
impl DatasetActorControl {
    fn stop(&self) {
        let _ = self.commands.try_send(DatasetLiveCommand::Close);
    }
}

/// Cooperative cancellation for an upstream client.  Dropping a browser
/// subscription must close the final Databento client, rather than aborting a
/// task and relying on socket drop semantics.
#[derive(Clone)]
struct LiveTaskControl {
    shutdown: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl LiveTaskControl {
    fn stop(&self) {
        if let Some(shutdown) = self
            .shutdown
            .lock()
            .expect("live shutdown lock poisoned")
            .take()
        {
            let _ = shutdown.send(());
        }
    }
}

#[cfg(feature = "databento-compat")]
fn resolved_stream_key(
    request: &crate::protocol::BarRequest,
    mapping: &SymbolMapping,
) -> ResolvedStreamKeyLike {
    ResolvedStreamKeyLike {
        dataset: request.dataset.clone(),
        requested_symbol: request.symbol.clone(),
        stype_in: request.stype_in,
        resolution: request.resolution,
        gap_policy: request.gap_policy_or_default(),
        resolved_symbol: mapping.resolved_symbol.clone(),
        instrument_id: mapping.instrument_id,
        source_schema: request.resolution.source_schema(),
    }
}

#[cfg(feature = "databento-compat")]
fn live_request(
    request: &crate::protocol::BarRequest,
    replay_start_seconds: i64,
    capacity: usize,
) -> LiveSubscriptionRequest {
    LiveSubscriptionRequest {
        dataset: request.dataset.clone(),
        symbol: request.symbol.clone(),
        stype_in: request.stype_in,
        schema: request.resolution.source_schema(),
        replay_start_seconds,
        output_capacity: std::num::NonZeroUsize::new(capacity)
            .expect("validated non-zero canonical queue capacity"),
        reconnect: ReconnectPolicy { max_attempts: 0 },
    }
}

#[cfg(feature = "databento-compat")]
async fn dataset_actor(
    state: &AppState,
    dataset: &str,
) -> Result<DatasetActorControl, ProviderErrorCode> {
    if let Some(existing) = state
        .live_sessions
        .lock()
        .expect("live session lock poisoned")
        .dataset_actors
        .get(dataset)
        .cloned()
    {
        return Ok(existing);
    }
    let key = state
        .live_api_key
        .as_ref()
        .ok_or(ProviderErrorCode::UpstreamUnavailable)?
        .to_string();
    let actor = DatabentoDatasetActor::connect(dataset.to_string(), key)
        .await
        .map_err(|_| ProviderErrorCode::UpstreamUnavailable)?;
    let (commands, command_receiver) = mpsc::channel(state.config.canonical_queue_capacity);
    let (events, mut event_receiver) = mpsc::channel(state.config.canonical_queue_capacity);
    let control = DatasetActorControl { commands };
    let registry = state.live_registry.clone();
    let dataset_for_events = dataset.to_string();
    let state_for_events = state.clone();
    tokio::spawn(async move {
        while let Some(DatasetLiveEvent { key, event }) = event_receiver.recv().await {
            let retain_for_replay = matches!(event, LiveEvent::Bar(_));
            let publish = registry
                .lock()
                .expect("live registry lock poisoned")
                .publish_with_replay_retention(&dataset_for_events, &key, event, retain_for_replay);
            for subscription_id in publish.slow_consumers {
                let state = state_for_events.clone();
                tokio::spawn(async move {
                    evict_slow_consumer(&state, &subscription_id).await;
                });
            }
        }
    });
    tokio::spawn(async move {
        let _ = actor.run(command_receiver, events).await;
    });

    let mut live = state
        .live_sessions
        .lock()
        .expect("live session lock poisoned");
    if let Some(existing) = live.dataset_actors.get(dataset).cloned() {
        control.stop();
        Ok(existing)
    } else {
        live.dataset_actors
            .insert(dataset.to_string(), control.clone());
        Ok(control)
    }
}

#[cfg(feature = "databento-compat")]
async fn attach_dataset_live(
    state: &AppState,
    request: &crate::protocol::BarRequest,
    mapping: &SymbolMapping,
    subscription_id: &str,
    replay_start_seconds: i64,
) -> Result<(mpsc::Receiver<LiveEvent>, ResolvedStreamKeyLike), ProviderErrorCode> {
    let key = resolved_stream_key(request, mapping);
    let lease = state
        .live_registry
        .lock()
        .expect("live registry lock poisoned")
        .acquire_with_replay_boundary(
            &request.dataset,
            key.clone(),
            subscription_id.to_string(),
            LiveEvent::ReplayCompleted,
        )
        .map_err(|error| match error {
            RegistryAcquireError::DatasetLimit => ProviderErrorCode::QuotaExceeded,
        })?;
    let actor = match dataset_actor(state, &request.dataset).await {
        Ok(actor) => actor,
        Err(error) => {
            release_dataset_live(state, &request.dataset, &key, subscription_id);
            return Err(error);
        }
    };
    if lease.canonical_added
        && actor
            .commands
            .send(DatasetLiveCommand::Add {
                key: key.clone(),
                request: live_request(
                    request,
                    replay_start_seconds,
                    state.config.canonical_queue_capacity,
                ),
            })
            .await
            .is_err()
    {
        release_dataset_live(state, &request.dataset, &key, subscription_id);
        return Err(ProviderErrorCode::UpstreamUnavailable);
    }
    Ok((lease.receiver, key))
}

pub async fn route_health_live() -> impl IntoResponse {
    crate::health::live().await
}

pub async fn route_health_ready() -> impl IntoResponse {
    crate::health::ready().await
}

pub async fn route_history_bars(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<HistoryRequest>,
) -> Response {
    if request.stype_in == SymbolType::Parent {
        return gateway_error(
            GatewayError::unsupported_parent_series("unsupported parent symbol"),
            headers,
            None,
        );
    }

    if let Err(error) = assert_request_bounds(request.from, request.to) {
        return gateway_error(error, headers, None);
    }
    if let Err(error) = assert_history_interval_cap(
        request.from,
        request.to,
        request.resolution.as_seconds(),
        state.config.history_max_intervals,
    ) {
        return gateway_error(error, headers, None);
    }

    let base_bars = state.history_source.get_bars(&request).await;
    let base_bars = match base_bars {
        Ok(value) => value,
        Err(error) => return gateway_error(error, headers, None),
    };

    let aggregated = match aggregate(
        &base_bars,
        request.from,
        request.to,
        &AggregationConfig::new(
            request.resolution.as_seconds(),
            request.gap_policy_or_default(),
        ),
    ) {
        Ok(value) => value,
        Err(_) => {
            return gateway_error(
                GatewayError::protocol("aggregation failed", ProviderErrorCode::ProtocolError),
                headers,
                None,
            );
        }
    };

    let mappings = match state
        .history_source
        .resolve_symbols(&ResolveRequest {
            v: request.v,
            dataset: request.dataset.clone(),
            symbols: vec![request.symbol.clone()],
            stype_in: request.stype_in,
            from: request.from,
            to: request.to,
        })
        .await
    {
        Ok(mappings) => match require_mapping(mappings) {
            Ok(mappings) => mappings,
            Err(error) => return gateway_error(error, headers, None),
        },
        Err(error) => return gateway_error(error, headers, None),
    };

    let (bars, volumes, metadata) = response_payload(&request, &aggregated, &mappings);
    let response = crate::protocol::BarPageResponse {
        v: request.v,
        request_id: request_id(&headers),
        bars,
        volumes,
        metadata,
    };

    (StatusCode::OK, Json(response)).into_response()
}

fn require_mapping(mappings: Vec<SymbolMapping>) -> Result<Vec<SymbolMapping>, GatewayError> {
    if mappings.is_empty() {
        return Err(GatewayError::protocol(
            "resolved symbol mapping is required for bar metadata",
            ProviderErrorCode::SymbolMappingFailed,
        ));
    }
    Ok(mappings)
}

pub async fn route_resolve(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<crate::protocol::ResolveRequest>,
) -> Response {
    match state.history_source.resolve_symbols(&request).await {
        Ok(mappings) => {
            let response = crate::protocol::ResolveResponse {
                v: request.v,
                request_id: request_id(&headers),
                mappings,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(error) => gateway_error(error, headers, None),
    }
}

pub async fn route_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SearchRequest>,
) -> Response {
    match state.history_source.search_symbols(&request).await {
        Ok(results) => {
            let response = SearchResponse {
                v: request.v,
                request_id: request_id(&headers),
                results,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(error) => gateway_error(error, headers, None),
    }
}

pub async fn route_dataset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(dataset): Path<String>,
) -> Response {
    match state.history_source.dataset_metadata(&dataset).await {
        Ok(metadata) => {
            let response = crate::protocol::DatasetResponse {
                v: 1,
                request_id: request_id(&headers),
                metadata,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(error) => gateway_error(error, headers, None),
    }
}

pub async fn route_live(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    if state.active_clients.load(Ordering::Acquire) >= state.config.max_clients {
        return (StatusCode::TOO_MANY_REQUESTS, "client limit reached").into_response();
    }
    let accepts_protocol = headers
        .get(SEC_WEBSOCKET_PROTOCOL)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .split(',')
        .any(|value| value.trim() == LIVE_SUBPROTOCOL);

    if !accepts_protocol {
        return (
            StatusCode::BAD_REQUEST,
            "missing required websocket protocol",
        )
            .into_response();
    }

    ws.protocols([LIVE_SUBPROTOCOL])
        .on_upgrade(|socket| handle_live(socket, state))
        .into_response()
}

async fn handle_live(socket: WebSocket, state: AppState) {
    let previous = state.active_clients.fetch_add(1, Ordering::AcqRel);
    if previous >= state.config.max_clients {
        state.active_clients.fetch_sub(1, Ordering::AcqRel);
        return;
    }
    let _client_guard = ActiveClientGuard(Arc::clone(&state.active_clients));
    let (mut writer, mut reader) = socket.split();
    let (outbound, mut outbound_rx) =
        mpsc::channel::<Message>(state.config.outbound_queue_capacity);
    let writer_task = tokio::spawn(async move {
        while let Some(message) = outbound_rx.recv().await {
            if writer.send(message).await.is_err() {
                break;
            }
        }
    });
    let mut connection_subscriptions = HashSet::new();
    // A heartbeat at connection establishment proves the negotiated protocol is
    // writable even before a subscription is accepted. Periodic upstream-driven
    // heartbeats belong behind the real live client boundary.
    let heartbeat = crate::protocol::ServerEvent::Heartbeat(crate::protocol::HeartbeatEvent {
        v: 1,
        server_time: unix_seconds(),
    });
    if outbound
        .send(Message::Text(json!(heartbeat).to_string().into()))
        .await
        .is_err()
    {
        return;
    }
    while let Some(Ok(message)) = reader.next().await {
        let payload = match message {
            Message::Text(payload) => payload,
            Message::Close(_) => break,
            Message::Ping(payload) => {
                let _ = outbound.send(Message::Pong(payload)).await;
                continue;
            }
            Message::Pong(_) => continue,
            Message::Binary(_) => {
                send_error(
                    &outbound,
                    1,
                    None,
                    None,
                    ProviderErrorCode::ProtocolError,
                    "binary websocket frames are not supported".to_string(),
                    false,
                )
                .await;
                break;
            }
        };

        if payload.len() > state.config.ws_frame_max_bytes {
            send_error(
                &outbound,
                1,
                None,
                None,
                ProviderErrorCode::InvalidRequest,
                "websocket frame exceeds configured limit".to_string(),
                false,
            )
            .await;
            continue;
        }

        let command = match parse_client_command(payload.as_bytes()) {
            Ok(command) => command,
            Err(error) => {
                let event = ErrorEvent {
                    v: 1,
                    command_id: None,
                    subscription_id: None,
                    error: ProviderError {
                        code: ProviderErrorCode::ProtocolError,
                        message: error.to_string(),
                        retryable: false,
                        details: Value::Object(Default::default()),
                    },
                };
                let _ = outbound
                    .send(Message::Text(
                        json!(crate::protocol::ServerEvent::Error(event))
                            .to_string()
                            .into(),
                    ))
                    .await;
                continue;
            }
        };

        match command {
            ClientCommand::SubscribeBars(command) => {
                if !ensure_subscription_capacity(
                    &connection_subscriptions,
                    &command.subscription_id,
                    state.config.max_subscriptions_per_client,
                    command.request.v,
                    &command.command_id,
                    &outbound,
                )
                .await
                {
                    continue;
                }
                let subscription_id = command.subscription_id.clone();
                process_subscribe(command, &state, &outbound).await;
                if state
                    .live_sessions
                    .lock()
                    .expect("live session lock poisoned")
                    .subscriptions
                    .contains_key(&subscription_id)
                {
                    connection_subscriptions.insert(subscription_id);
                }
            }
            ClientCommand::OpenBars(command) => {
                if !ensure_subscription_capacity(
                    &connection_subscriptions,
                    &command.subscription_id,
                    state.config.max_subscriptions_per_client,
                    command.request.v,
                    &command.command_id,
                    &outbound,
                )
                .await
                {
                    continue;
                }
                let subscription_id = command.subscription_id.clone();
                process_open(command, &state, &outbound).await;
                if state
                    .live_sessions
                    .lock()
                    .expect("live session lock poisoned")
                    .subscriptions
                    .contains_key(&subscription_id)
                {
                    connection_subscriptions.insert(subscription_id);
                }
            }
            ClientCommand::ResumeBars(command) => {
                if !ensure_subscription_capacity(
                    &connection_subscriptions,
                    &command.subscription_id,
                    state.config.max_subscriptions_per_client,
                    command.request.v,
                    &command.command_id,
                    &outbound,
                )
                .await
                {
                    continue;
                }
                let subscription_id = command.subscription_id.clone();
                process_resume(command, &state, &outbound).await;
                if state
                    .live_sessions
                    .lock()
                    .expect("live session lock poisoned")
                    .subscriptions
                    .contains_key(&subscription_id)
                {
                    connection_subscriptions.insert(subscription_id);
                }
            }
            ClientCommand::Unsubscribe(command) => {
                connection_subscriptions.remove(&command.subscription_id);
                remove_subscription(&state, &command.subscription_id);
                let event = crate::protocol::ServerEvent::Unsubscribed(UnsubscribedEvent {
                    v: 1,
                    command_id: command.command_id,
                    subscription_id: command.subscription_id,
                });
                let _ = outbound
                    .send(Message::Text(json!(event).to_string().into()))
                    .await;
            }
            ClientCommand::Cancel(command) => {
                connection_subscriptions.remove(&command.subscription_id);
                remove_subscription(&state, &command.subscription_id);
                let event = crate::protocol::ServerEvent::Cancelled(CancelledEvent {
                    v: 1,
                    command_id: command.command_id,
                    target_command_id: command.target_command_id,
                    subscription_id: command.subscription_id,
                });
                let _ = outbound
                    .send(Message::Text(json!(event).to_string().into()))
                    .await;
            }
        }
    }
    drop(outbound);
    let _ = writer_task.await;
}

struct ActiveClientGuard(Arc<AtomicUsize>);

impl Drop for ActiveClientGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

async fn ensure_subscription_capacity(
    active: &HashSet<String>,
    subscription_id: &str,
    maximum: usize,
    version: u8,
    command_id: &str,
    socket: &Outbound,
) -> bool {
    if active.contains(subscription_id) || active.len() < maximum {
        return true;
    }
    send_error(
        socket,
        version,
        Some(command_id.to_string()),
        Some(subscription_id.to_string()),
        ProviderErrorCode::QuotaExceeded,
        "subscription limit reached".to_string(),
        false,
    )
    .await;
    false
}

fn remove_subscription(state: &AppState, subscription_id: &str) {
    let removed = state
        .live_sessions
        .lock()
        .expect("live session lock poisoned")
        .subscriptions
        .remove(subscription_id);
    if let Some(subscription) = removed {
        #[cfg(feature = "databento-compat")]
        if let Some(key) = subscription.registry_key.as_ref() {
            release_dataset_live(state, &subscription.request.dataset, key, subscription_id);
        }
        if let Some(task) = subscription.live_task {
            task.stop();
        }
    }
}

/// Notifies and evicts a subscription the live registry reported as a slow
/// consumer (its bounded per-subscriber queue was full). Without this, a
/// stalled downstream keeps silently missing bars while the client still
/// believes it is subscribed and live.
#[cfg(feature = "databento-compat")]
async fn evict_slow_consumer(state: &AppState, subscription_id: &str) {
    let notify = state
        .live_sessions
        .lock()
        .expect("live session lock poisoned")
        .subscriptions
        .get(subscription_id)
        .map(|subscription| (subscription.outbound.clone(), subscription.request.v));
    if let Some((outbound, v)) = notify {
        send_error(
            &outbound,
            v,
            None,
            Some(subscription_id.to_string()),
            ProviderErrorCode::SlowConsumer,
            "downstream fell too far behind and was disconnected".to_string(),
            false,
        )
        .await;
    }
    remove_subscription(state, subscription_id);
}

#[cfg(feature = "databento-compat")]
fn release_dataset_live(
    state: &AppState,
    dataset: &str,
    key: &ResolvedStreamKeyLike,
    subscription_id: &str,
) {
    let release = state
        .live_registry
        .lock()
        .expect("live registry lock poisoned")
        .release(dataset, key, subscription_id);
    if release.dataset_released {
        if let Some(actor) = state
            .live_sessions
            .lock()
            .expect("live session lock poisoned")
            .dataset_actors
            .remove(dataset)
        {
            actor.stop();
        }
    }
}

fn unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn live_handoff_boundaries(
    requested_from: i64,
    requested_to: i64,
    available_to: Option<i64>,
    source_interval: i64,
) -> (i64, i64) {
    let historical_to = available_to
        .map(|value| value.min(requested_to))
        .unwrap_or(requested_to)
        .max(requested_from);
    let replay_start = historical_to
        .saturating_sub(source_interval)
        .max(requested_from);
    (historical_to, replay_start)
}

fn historical_resolution_range(
    requested_from: i64,
    live_edge: i64,
    available_to: Option<i64>,
    source_interval: i64,
) -> (i64, i64) {
    let to = available_to
        .map(|value| value.min(live_edge))
        .unwrap_or(live_edge);
    let from = requested_from.min(to.saturating_sub(source_interval));
    (from, to)
}

fn require_live_edge(
    requested_from: i64,
    requested_to: i64,
    now: i64,
    source_interval: i64,
) -> Result<i64, GatewayError> {
    let live_edge = now - now.rem_euclid(source_interval);
    const TRANSIT_GRACE_SECONDS: i64 = 5;
    let earliest_accepted_to = live_edge.saturating_sub(TRANSIT_GRACE_SECONDS);
    if requested_to < earliest_accepted_to || requested_to > now || requested_from >= live_edge {
        return Err(GatewayError::invalid_range(
            "open_bars requires a range ending in the current source interval",
        ));
    }
    Ok(live_edge)
}

async fn process_subscribe(
    command: crate::protocol::SubscribeBarsCommand,
    state: &AppState,
    socket: &Outbound,
) {
    if command.request.stype_in == SymbolType::Parent {
        let event = crate::protocol::ServerEvent::Error(ErrorEvent {
            v: 1,
            command_id: Some(command.command_id),
            subscription_id: Some(command.subscription_id),
            error: ProviderError {
                code: ProviderErrorCode::UnsupportedParentSeries,
                message: "unsupported parent symbol".to_string(),
                retryable: false,
                details: Value::Object(Default::default()),
            },
        });
        let _ = socket
            .send(Message::Text(json!(event).to_string().into()))
            .await;
        return;
    }

    let now = unix_seconds();
    let source_interval = schema_interval_seconds(&command.request.resolution.source_schema());
    let available_to = state
        .history_source
        .dataset_metadata(&command.request.dataset)
        .await
        .ok()
        .and_then(|metadata| metadata.available_to);
    let (resolution_from, resolution_to) = historical_resolution_range(
        now.saturating_sub(source_interval),
        now,
        available_to,
        source_interval,
    );
    let mappings = match state
        .history_source
        .resolve_symbols(&crate::protocol::ResolveRequest {
            v: command.request.v,
            dataset: command.request.dataset.clone(),
            symbols: vec![command.request.symbol.clone()],
            stype_in: command.request.stype_in,
            from: resolution_from,
            to: resolution_to,
        })
        .await
    {
        Ok(mappings) if !mappings.is_empty() => mappings,
        Ok(_) => {
            send_error(
                socket,
                command.request.v,
                Some(command.command_id),
                Some(command.subscription_id),
                ProviderErrorCode::SymbolMappingFailed,
                "symbol resolution returned no mapping".to_string(),
                false,
            )
            .await;
            return;
        }
        Err(error) => {
            send_error(
                socket,
                command.request.v,
                Some(command.command_id),
                Some(command.subscription_id),
                error.error_body().code,
                error.error_body().message,
                error.error_body().retryable,
            )
            .await;
            return;
        }
    };

    #[cfg(feature = "databento-compat")]
    let (live_task, registry_key) = match state.live_api_key.as_ref() {
        Some(_) => match attach_dataset_live(
            state,
            &command.request,
            mappings.first().expect("non-empty mapping checked above"),
            &command.subscription_id,
            now.saturating_sub(source_interval),
        )
        .await
        {
            Ok((receiver, registry_key)) => {
                let outbound = socket.clone();
                let request = history_request_for_subscription(&command.request);
                let subscription_id = command.subscription_id.clone();
                let mapping = mappings.first().cloned();
                tokio::spawn(async move {
                    forward_live_events(
                        receiver,
                        outbound,
                        request,
                        subscription_id,
                        mapping,
                        SteadyAggregation::default(),
                    )
                    .await;
                });
                (None, Some(registry_key))
            }
            Err(code) => {
                send_error(
                    socket,
                    command.request.v,
                    Some(command.command_id),
                    Some(command.subscription_id),
                    code,
                    "live subscription could not be started".to_string(),
                    true,
                )
                .await;
                return;
            }
        },
        None => (None, None),
    };
    #[cfg(not(feature = "databento-compat"))]
    let live_task: Option<LiveTaskControl> = None;

    state
        .live_sessions
        .lock()
        .expect("live session lock poisoned")
        .subscriptions
        .insert(
            command.subscription_id.clone(),
            ActiveSubscription {
                mappings: mappings.clone(),
                request: command.request.clone(),
                live_task,
                outbound: socket.clone(),
                #[cfg(feature = "databento-compat")]
                registry_key,
            },
        );

    let event = crate::protocol::ServerEvent::Subscribed(SubscribedEvent {
        v: command.request.v,
        command_id: command.command_id,
        subscription_id: command.subscription_id,
        state: ProviderState::Live,
        resolved_symbols: mappings,
    });
    let _ = socket
        .send(Message::Text(json!(event).to_string().into()))
        .await;
}

async fn process_open(
    command: crate::protocol::OpenBarsCommand,
    state: &AppState,
    socket: &Outbound,
) {
    if command.request.stype_in == SymbolType::Parent {
        let event = crate::protocol::ServerEvent::Error(ErrorEvent {
            v: command.request.v,
            command_id: Some(command.command_id),
            subscription_id: Some(command.subscription_id.clone()),
            error: ProviderError {
                code: ProviderErrorCode::UnsupportedParentSeries,
                message: "unsupported parent symbol".to_string(),
                retryable: false,
                details: Value::Object(Default::default()),
            },
        });
        let _ = socket
            .send(Message::Text(json!(event).to_string().into()))
            .await;
        return;
    }

    let source_interval = schema_interval_seconds(&command.request.resolution.source_schema());
    let live_edge = match require_live_edge(
        command.request.from,
        command.request.to,
        unix_seconds(),
        source_interval,
    ) {
        Ok(value) => value,
        Err(error) => {
            send_error(
                socket,
                command.request.v,
                Some(command.command_id),
                Some(command.subscription_id),
                error.error_body().code,
                error.error_body().message,
                false,
            )
            .await;
            return;
        }
    };

    if let Err(error) = assert_history_interval_cap(
        command.request.from,
        command.request.to,
        command.request.resolution.as_seconds(),
        state.config.history_max_intervals,
    ) {
        send_error(
            socket,
            command.request.v,
            Some(command.command_id),
            Some(command.subscription_id),
            error.error_body().code,
            error.error_body().message,
            false,
        )
        .await;
        return;
    }

    let available_to = state
        .history_source
        .dataset_metadata(&command.request.dataset)
        .await
        .ok()
        .and_then(|metadata| metadata.available_to);
    let (resolution_from, resolution_to) = historical_resolution_range(
        command.request.from,
        live_edge,
        available_to,
        source_interval,
    );

    let mappings = match state
        .history_source
        .resolve_symbols(&ResolveRequest {
            v: command.request.v,
            dataset: command.request.dataset.clone(),
            symbols: vec![command.request.symbol.clone()],
            stype_in: command.request.stype_in,
            from: resolution_from,
            to: resolution_to,
        })
        .await
    {
        Ok(values) if !values.is_empty() => values,
        Ok(_) => {
            send_error(
                socket,
                command.request.v,
                Some(command.command_id),
                Some(command.subscription_id),
                ProviderErrorCode::SymbolMappingFailed,
                "symbol resolution returned no mapping".to_string(),
                false,
            )
            .await;
            return;
        }
        Err(error) => {
            let event = crate::protocol::ServerEvent::Error(ErrorEvent {
                v: command.request.v,
                command_id: Some(command.command_id),
                subscription_id: Some(command.subscription_id),
                error: ProviderError {
                    code: error.error_body().code,
                    message: error.error_body().message,
                    retryable: error.error_body().retryable,
                    details: error.error_body().details,
                },
            });
            let _ = socket
                .send(Message::Text(json!(event).to_string().into()))
                .await;
            return;
        }
    };

    let prior_subscription = {
        state
            .live_sessions
            .lock()
            .expect("live session lock poisoned")
            .subscriptions
            .get(&command.subscription_id)
            .cloned()
    };
    if let Some(previous) = prior_subscription {
        if !previous.matches(&mappings) {
            let event = crate::protocol::ServerEvent::Error(ErrorEvent {
                v: command.request.v,
                command_id: Some(command.command_id),
                subscription_id: Some(command.subscription_id),
                error: ProviderError {
                    code: ProviderErrorCode::ResolvedInstrumentChanged,
                    message: "continuous symbol resolved to a different instrument".to_string(),
                    retryable: false,
                    details: Value::Object(Default::default()),
                },
            });
            let _ = socket
                .send(Message::Text(json!(event).to_string().into()))
                .await;
            return;
        }
    }

    let subscribed = crate::protocol::ServerEvent::Subscribed(SubscribedEvent {
        v: command.request.v,
        command_id: command.command_id.clone(),
        subscription_id: command.subscription_id.clone(),
        state: ProviderState::Replaying,
        resolved_symbols: mappings.clone(),
    });
    let _ = socket
        .send(Message::Text(json!(subscribed).to_string().into()))
        .await;

    let (historical_to, replay_start) = live_handoff_boundaries(
        command.request.from,
        live_edge,
        available_to,
        source_interval,
    );
    #[cfg(not(feature = "databento-compat"))]
    let _ = replay_start;
    let mut historical_request = command.request.clone();
    historical_request.to = historical_to;

    // Register upstream replay before history so records which arrive during
    // history loading remain buffered until the ReplayCompleted boundary.
    let subscription_request = bar_request_from_history(&command.request);
    #[cfg(feature = "databento-compat")]
    let mut live_receiver = match state.live_api_key.as_ref() {
        Some(_) => match attach_dataset_live(
            state,
            &subscription_request,
            mappings.first().expect("open mapping is required"),
            &command.subscription_id,
            replay_start,
        )
        .await
        {
            Ok(value) => Some(value),
            Err(code) => {
                send_error(
                    socket,
                    command.request.v,
                    Some(command.command_id),
                    Some(command.subscription_id),
                    code,
                    "live replay could not be started".to_string(),
                    true,
                )
                .await;
                return;
            }
        },
        None => None,
    };

    #[cfg(feature = "databento-compat")]
    let replay_task = live_receiver.take().map(|(mut receiver, registry_key)| {
        let task = tokio::spawn(async move {
            let replay = wait_for_replay(&mut receiver).await;
            (replay, receiver)
        });
        (task, registry_key)
    });

    let bars = match if historical_request.from < historical_request.to {
        state.history_source.get_bars(&historical_request).await
    } else {
        Ok(Vec::new())
    } {
        Ok(value) => value,
        Err(error) => {
            #[cfg(feature = "databento-compat")]
            if let Some((task, registry_key)) = replay_task {
                task.abort();
                release_dataset_live(
                    state,
                    &command.request.dataset,
                    &registry_key,
                    &command.subscription_id,
                );
            }
            let event = crate::protocol::ServerEvent::Error(ErrorEvent {
                v: command.request.v,
                command_id: Some(command.command_id),
                subscription_id: Some(command.subscription_id.clone()),
                error: ProviderError {
                    code: error.error_body().code,
                    message: error.error_body().message,
                    retryable: error.error_body().retryable,
                    details: error.error_body().details,
                },
            });
            let _ = socket
                .send(Message::Text(json!(event).to_string().into()))
                .await;
            return;
        }
    };

    #[cfg(feature = "databento-compat")]
    let bars = if let Some((task, registry_key)) = replay_task {
        let (replay, receiver) = match task.await {
            Ok(value) => value,
            Err(_) => {
                release_dataset_live(
                    state,
                    &command.request.dataset,
                    &registry_key,
                    &command.subscription_id,
                );
                send_error(
                    socket,
                    command.request.v,
                    Some(command.command_id),
                    Some(command.subscription_id),
                    ProviderErrorCode::ReplayUnavailable,
                    "live replay did not reach a safe handoff boundary".to_string(),
                    true,
                )
                .await;
                return;
            }
        };
        let buffered = match replay {
            Ok(value) => value,
            Err(code) => {
                release_dataset_live(
                    state,
                    &command.request.dataset,
                    &registry_key,
                    &command.subscription_id,
                );
                send_error(
                    socket,
                    command.request.v,
                    Some(command.command_id),
                    Some(command.subscription_id),
                    code,
                    "live replay did not reach a safe handoff boundary".to_string(),
                    true,
                )
                .await;
                return;
            }
        };
        let mut merged = bars;
        merge_base_bars(&mut merged, buffered);
        (merged, Some((receiver, registry_key)))
    } else {
        (bars, None)
    };
    #[cfg(feature = "databento-compat")]
    let (bars, mut live_after_snapshot) = bars;

    let aggregated = match aggregate(
        &bars,
        command.request.from,
        command.request.to,
        &AggregationConfig::new(
            command.request.resolution.as_seconds(),
            command.request.gap_policy_or_default(),
        ),
    ) {
        Ok(value) => value,
        Err(_) => {
            #[cfg(feature = "databento-compat")]
            if let Some((_, registry_key)) = live_after_snapshot.take() {
                release_dataset_live(
                    state,
                    &command.request.dataset,
                    &registry_key,
                    &command.subscription_id,
                );
            }
            let event = crate::protocol::ServerEvent::Error(ErrorEvent {
                v: command.request.v,
                command_id: Some(command.command_id),
                subscription_id: Some(command.subscription_id),
                error: ProviderError {
                    code: ProviderErrorCode::ProtocolError,
                    message: "failed to aggregate data".to_string(),
                    retryable: false,
                    details: Value::Object(Default::default()),
                },
            });
            let _ = socket
                .send(Message::Text(json!(event).to_string().into()))
                .await;
            return;
        }
    };

    let (bars, volumes, metadata) = response_payload(&command.request, &aggregated, &mappings);
    let snapshot = crate::protocol::ServerEvent::Snapshot(SnapshotEvent {
        v: command.request.v,
        subscription_id: command.subscription_id.clone(),
        bars,
        volumes,
        metadata,
    });
    let _ = socket
        .send(Message::Text(json!(snapshot).to_string().into()))
        .await;

    let status = crate::protocol::ServerEvent::Status(StatusEvent {
        v: command.request.v,
        subscription_id: command.subscription_id.clone(),
        state: ProviderState::Live,
        retryable: false,
        reason: None,
    });
    let _ = socket
        .send(Message::Text(json!(status).to_string().into()))
        .await;

    #[cfg(feature = "databento-compat")]
    let registry_key = live_after_snapshot
        .as_ref()
        .map(|(_, registry_key)| registry_key.clone());
    #[cfg(feature = "databento-compat")]
    if let Some((receiver, _)) = live_after_snapshot {
        let outbound = socket.clone();
        let request = command.request.clone();
        let subscription_id = command.subscription_id.clone();
        let mapping = mappings.first().cloned();
        tokio::spawn(async move {
            forward_live_events(
                receiver,
                outbound,
                request,
                subscription_id,
                mapping,
                SteadyAggregation::default(),
            )
            .await;
        });
    }
    #[cfg(feature = "databento-compat")]
    let live_task: Option<LiveTaskControl> = None;
    #[cfg(not(feature = "databento-compat"))]
    let live_task: Option<LiveTaskControl> = None;

    state
        .live_sessions
        .lock()
        .expect("live session lock poisoned")
        .subscriptions
        .insert(
            command.subscription_id,
            ActiveSubscription {
                mappings,
                request: subscription_request,
                live_task,
                outbound: socket.clone(),
                #[cfg(feature = "databento-compat")]
                registry_key,
            },
        );
}

#[cfg(feature = "databento-compat")]
async fn wait_for_replay(
    receiver: &mut mpsc::Receiver<LiveEvent>,
) -> Result<Vec<NormalizedBaseBar>, ProviderErrorCode> {
    const REPLAY_BOUNDARY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(45);
    let mut buffered = Vec::new();
    loop {
        let event = tokio::time::timeout(REPLAY_BOUNDARY_TIMEOUT, receiver.recv())
            .await
            .map_err(|_| ProviderErrorCode::ReplayUnavailable)?
            .ok_or(ProviderErrorCode::ReplayUnavailable)?;
        match event {
            LiveEvent::Bar(bar) => buffered.push(bar),
            LiveEvent::SymbolMapping { .. } => {}
            LiveEvent::ReplayCompleted => return Ok(buffered),
            LiveEvent::Heartbeat => {}
            LiveEvent::Ended => return Err(ProviderErrorCode::ReplayUnavailable),
            LiveEvent::Failure(LiveBoundaryError::SlowConsumer) => {
                return Err(ProviderErrorCode::SlowConsumer)
            }
            LiveEvent::Failure(_) => return Err(ProviderErrorCode::UpstreamUnavailable),
        }
    }
}

/// Establishes the reconnect boundary.  The upstream subscription deliberately
/// begins one native source interval before `resume_from`; overlap records are
/// consumed only to reach `ReplayCompleted` and are never forwarded. Equal-time
/// revisions and newer records are deduplicated by source-bar identity, then
/// sorted before their chart updates are emitted.
#[cfg(feature = "databento-compat")]
async fn wait_for_resume_replay(
    receiver: &mut mpsc::Receiver<LiveEvent>,
    resume_from: i64,
) -> Result<Vec<NormalizedBaseBar>, ProviderErrorCode> {
    let replay = wait_for_replay(receiver).await?;
    let mut by_key = HashMap::new();
    for bar in replay.into_iter().filter(|bar| bar.time >= resume_from) {
        by_key.insert((bar.dataset.clone(), bar.instrument_id, bar.time), bar);
    }
    let mut filtered = by_key.into_values().collect::<Vec<_>>();
    filtered.sort_by_key(|bar| (bar.time, bar.instrument_id));
    Ok(filtered)
}

#[cfg(feature = "databento-compat")]
fn merge_base_bars(history: &mut Vec<NormalizedBaseBar>, replay: Vec<NormalizedBaseBar>) {
    let mut by_key = HashMap::with_capacity(history.len() + replay.len());
    for bar in history.drain(..).chain(replay) {
        by_key.insert((bar.dataset.clone(), bar.instrument_id, bar.time), bar);
    }
    let mut merged = by_key.into_values().collect::<Vec<_>>();
    merged.sort_by_key(|bar| (bar.time, bar.instrument_id));
    *history = merged;
}

#[cfg(feature = "databento-compat")]
async fn forward_live_events(
    mut receiver: mpsc::Receiver<LiveEvent>,
    outbound: Outbound,
    request: HistoryRequest,
    subscription_id: String,
    mapping: Option<SymbolMapping>,
    mut steady: SteadyAggregation,
) {
    // Retain only the source bars needed to rebuild the active target bucket.
    // DBN may revise the currently open source bar at the same timestamp, so
    // the map replaces it before the aggregate is emitted as an equal-time
    // chart update.
    while let Some(event) = receiver.recv().await {
        let result = match event {
            LiveEvent::Bar(base) => {
                send_live_bar(
                    &outbound,
                    &request,
                    &subscription_id,
                    mapping.as_ref(),
                    base,
                    &mut steady,
                )
                .await
            }
            LiveEvent::SymbolMapping {
                requested_symbol,
                resolved_symbol,
                instrument_id,
                effective_from,
            } => outbound
                .try_send(Message::Text(
                    json!(crate::protocol::ServerEvent::SymbolMapping(
                        crate::protocol::SymbolMappingEvent {
                            v: request.v,
                            subscription_id: subscription_id.clone(),
                            requested_symbol,
                            resolved_symbol,
                            instrument_id,
                            effective_from,
                        }
                    ))
                    .to_string()
                    .into(),
                ))
                .map_err(|_| ProviderErrorCode::SlowConsumer),
            LiveEvent::Heartbeat => outbound
                .try_send(Message::Text(
                    json!(crate::protocol::ServerEvent::Heartbeat(
                        crate::protocol::HeartbeatEvent {
                            v: request.v,
                            server_time: unix_seconds()
                        }
                    ))
                    .to_string()
                    .into(),
                ))
                .map_err(|_| ProviderErrorCode::SlowConsumer),
            LiveEvent::ReplayCompleted => Ok(()),
            LiveEvent::Ended => Err(ProviderErrorCode::UpstreamUnavailable),
            LiveEvent::Failure(LiveBoundaryError::SlowConsumer) => {
                Err(ProviderErrorCode::SlowConsumer)
            }
            LiveEvent::Failure(LiveBoundaryError::ResolvedInstrumentChanged) => {
                Err(ProviderErrorCode::ResolvedInstrumentChanged)
            }
            LiveEvent::Failure(_) => Err(ProviderErrorCode::UpstreamUnavailable),
        };
        if let Err(code) = result {
            let _ = outbound.try_send(Message::Text(
                json!(crate::protocol::ServerEvent::Error(ErrorEvent {
                    v: request.v,
                    command_id: None,
                    subscription_id: Some(subscription_id.clone()),
                    error: ProviderError {
                        code,
                        message: "live stream stopped".to_string(),
                        retryable: code == ProviderErrorCode::UpstreamUnavailable,
                        details: Value::Object(Default::default())
                    },
                }))
                .to_string()
                .into(),
            ));
            return;
        }
    }
}

#[cfg(feature = "databento-compat")]
async fn send_live_bar(
    outbound: &Outbound,
    request: &HistoryRequest,
    subscription_id: &str,
    mapping: Option<&SymbolMapping>,
    base: NormalizedBaseBar,
    steady: &mut SteadyAggregation,
) -> Result<(), ProviderErrorCode> {
    let mapping = mapping.ok_or(ProviderErrorCode::SymbolMappingFailed)?;
    if request.resolution.source_schema() != base.schema {
        return Ok(());
    }
    let target = steady.update(
        base,
        request.resolution.as_seconds(),
        request.gap_policy_or_default(),
    )?;
    let event = crate::protocol::ServerEvent::Bar(BarEvent {
        v: request.v,
        subscription_id: subscription_id.to_string(),
        data: ChartBar::Candlestick {
            time: target.time,
            open: price_to_f64(target.open),
            high: price_to_f64(target.high),
            low: price_to_f64(target.low),
            close: price_to_f64(target.close),
        },
        volume: (target.volume > 0).then_some(VolumePoint {
            time: target.time,
            value: target.volume as f64,
            color: None,
        }),
        meta: BarMetadata {
            dataset: request.dataset.clone(),
            requested_symbol: request.symbol.clone(),
            resolved_symbol: mapping.resolved_symbol.clone(),
            instrument_id: mapping.instrument_id,
            source_schema: request.resolution.source_schema(),
            synthetic: false,
        },
    });
    outbound
        .try_send(Message::Text(json!(event).to_string().into()))
        .map_err(|_| ProviderErrorCode::SlowConsumer)
}

#[cfg(feature = "databento-compat")]
#[derive(Default)]
struct SteadyAggregation {
    // Native source interval start -> most recently received revision.
    base: HashMap<(i64, i64), NormalizedBaseBar>,
}

#[cfg(feature = "databento-compat")]
impl SteadyAggregation {
    fn update(
        &mut self,
        bar: NormalizedBaseBar,
        target_seconds: i64,
        gap_policy: crate::protocol::GapPolicy,
    ) -> Result<AggregatedBar, ProviderErrorCode> {
        let bucket = bar.time.div_euclid(target_seconds) * target_seconds;
        let instrument_id = bar.instrument_id;
        self.base.insert((instrument_id, bar.time), bar);
        // A bounded target-bucket cache: earlier bars can never mutate this
        // candle and are unnecessary for steady state output.
        self.base.retain(|(_, time), _| *time >= bucket);
        let mut components = self
            .base
            .values()
            .filter(|candidate| {
                candidate.instrument_id == instrument_id
                    && candidate.time >= bucket
                    && candidate.time < bucket.saturating_add(target_seconds)
            })
            .cloned()
            .collect::<Vec<_>>();
        components.sort_by_key(|candidate| candidate.time);
        aggregate(
            &components,
            bucket,
            bucket.saturating_add(target_seconds),
            &AggregationConfig::new(target_seconds, gap_policy),
        )
        .map_err(|_| ProviderErrorCode::ProtocolError)?
        .into_iter()
        .next()
        .ok_or(ProviderErrorCode::ProtocolError)
    }
}

async fn send_error(
    socket: &Outbound,
    v: u8,
    command_id: Option<String>,
    subscription_id: Option<String>,
    code: ProviderErrorCode,
    message: String,
    retryable: bool,
) {
    let event = crate::protocol::ServerEvent::Error(ErrorEvent {
        v,
        command_id,
        subscription_id,
        error: ProviderError {
            code,
            message,
            retryable,
            details: Value::Object(Default::default()),
        },
    });
    let _ = socket
        .send(Message::Text(json!(event).to_string().into()))
        .await;
}

async fn process_resume(
    command: crate::protocol::ResumeBarsCommand,
    state: &AppState,
    socket: &Outbound,
) {
    let previous = state
        .live_sessions
        .lock()
        .expect("live session lock poisoned")
        .subscriptions
        .get(&command.subscription_id)
        .cloned();
    let Some(previous) = previous else {
        let event = crate::protocol::ServerEvent::Error(ErrorEvent {
            v: command.request.v,
            command_id: Some(command.command_id),
            subscription_id: Some(command.subscription_id),
            error: ProviderError {
                code: ProviderErrorCode::InvalidRequest,
                message: "unknown subscription".to_string(),
                retryable: false,
                details: Value::Object(Default::default()),
            },
        });
        let _ = socket
            .send(Message::Text(json!(event).to_string().into()))
            .await;
        return;
    };

    if previous.request.dataset != command.request.dataset
        || previous.request.symbol != command.request.symbol
        || previous.request.stype_in != command.request.stype_in
        || previous.request.resolution != command.request.resolution
        || previous.request.gap_policy != command.request.gap_policy
    {
        send_error(
            socket,
            command.request.v,
            Some(command.command_id),
            Some(command.subscription_id),
            ProviderErrorCode::InvalidRequest,
            "resume request does not match the established subscription".to_string(),
            false,
        )
        .await;
        return;
    }

    let now = unix_seconds();
    let source_interval = schema_interval_seconds(&command.request.resolution.source_schema());
    let available_to = state
        .history_source
        .dataset_metadata(&command.request.dataset)
        .await
        .ok()
        .and_then(|metadata| metadata.available_to);
    let (resolution_from, resolution_to) =
        historical_resolution_range(command.resume_from, now, available_to, source_interval);

    let mappings = match state
        .history_source
        .resolve_symbols(&ResolveRequest {
            v: command.request.v,
            dataset: command.request.dataset.clone(),
            symbols: vec![command.request.symbol.clone()],
            stype_in: command.request.stype_in,
            from: resolution_from,
            to: resolution_to,
        })
        .await
    {
        Ok(mappings) if !mappings.is_empty() => mappings,
        Ok(_) => {
            send_error(
                socket,
                command.request.v,
                Some(command.command_id),
                Some(command.subscription_id),
                ProviderErrorCode::SymbolMappingFailed,
                "symbol resolution returned no mapping".to_string(),
                false,
            )
            .await;
            return;
        }
        Err(error) => {
            send_error(
                socket,
                command.request.v,
                Some(command.command_id),
                Some(command.subscription_id),
                error.error_body().code,
                error.error_body().message,
                error.error_body().retryable,
            )
            .await;
            return;
        }
    };

    if !previous.matches(&mappings) {
        remove_subscription(state, &command.subscription_id);
        send_error(
            socket,
            command.request.v,
            Some(command.command_id),
            Some(command.subscription_id),
            ProviderErrorCode::ResolvedInstrumentChanged,
            "continuous symbol resolved to a different instrument".to_string(),
            false,
        )
        .await;
        return;
    }

    // Attach before the last emitted time, then keep its receiver private until
    // ReplayCompleted establishes an ordered resume boundary.
    #[cfg(feature = "databento-compat")]
    let mut resume_live = if state.live_api_key.is_some() {
        if let Some(old_key) = previous.registry_key.as_ref() {
            release_dataset_live(
                state,
                &command.request.dataset,
                old_key,
                &command.subscription_id,
            );
        }
        let attached = match attach_dataset_live(
            state,
            &command.request,
            mappings.first().expect("resume mapping is required"),
            &command.subscription_id,
            command.resume_from.saturating_sub(schema_interval_seconds(
                &command.request.resolution.source_schema(),
            )),
        )
        .await
        {
            Ok(value) => value,
            Err(code) => {
                send_error(
                    socket,
                    command.request.v,
                    Some(command.command_id),
                    Some(command.subscription_id),
                    code,
                    "live subscription could not resume".to_string(),
                    true,
                )
                .await;
                return;
            }
        };
        Some(attached)
    } else {
        None
    };

    let subscribed = crate::protocol::ServerEvent::Subscribed(SubscribedEvent {
        v: command.request.v,
        command_id: command.command_id.clone(),
        subscription_id: command.subscription_id.clone(),
        state: ProviderState::Replaying,
        resolved_symbols: mappings.clone(),
    });
    let _ = socket
        .send(Message::Text(json!(subscribed).to_string().into()))
        .await;

    #[cfg(feature = "databento-compat")]
    if let Some((mut receiver, registry_key)) = resume_live.take() {
        let replay = match wait_for_resume_replay(&mut receiver, command.resume_from).await {
            Ok(replay) => replay,
            Err(code) => {
                release_dataset_live(
                    state,
                    &command.request.dataset,
                    &registry_key,
                    &command.subscription_id,
                );
                send_error(
                    socket,
                    command.request.v,
                    Some(command.command_id),
                    Some(command.subscription_id),
                    code,
                    "live replay did not reach a safe resume boundary".to_string(),
                    true,
                )
                .await;
                return;
            }
        };
        let request = history_request_for_subscription(&command.request);
        let mapping = mappings.first().cloned();
        let mut steady = SteadyAggregation::default();
        for base in replay {
            if let Err(code) = send_live_bar(
                socket,
                &request,
                &command.subscription_id,
                mapping.as_ref(),
                base,
                &mut steady,
            )
            .await
            {
                release_dataset_live(
                    state,
                    &command.request.dataset,
                    &registry_key,
                    &command.subscription_id,
                );
                send_error(
                    socket,
                    command.request.v,
                    Some(command.command_id),
                    Some(command.subscription_id),
                    code,
                    "resume consumer became unavailable".to_string(),
                    false,
                )
                .await;
                return;
            }
        }
        let outbound = socket.clone();
        let subscription_id = command.subscription_id.clone();
        tokio::spawn(async move {
            forward_live_events(
                receiver,
                outbound,
                request,
                subscription_id,
                mapping,
                steady,
            )
            .await;
        });
        let mut sessions = state
            .live_sessions
            .lock()
            .expect("live session lock poisoned");
        let subscription = sessions
            .subscriptions
            .get_mut(&command.subscription_id)
            .expect("resume subscription was checked above");
        subscription.registry_key = Some(registry_key);
        // A resume attaches a new WebSocket connection to the stable
        // subscription id; refresh the outbound handle so a later
        // slow-consumer eviction targets the live socket, not the one from
        // the connection that was replaced.
        subscription.outbound = socket.clone();
    }

    let status = crate::protocol::ServerEvent::Status(StatusEvent {
        v: command.request.v,
        subscription_id: command.subscription_id,
        state: ProviderState::Live,
        retryable: false,
        reason: Some(crate::protocol::StatusReason::ReplayCompleted),
    });
    let _ = socket
        .send(Message::Text(json!(status).to_string().into()))
        .await;
}

fn bar_request_from_history(request: &HistoryRequest) -> crate::protocol::BarRequest {
    crate::protocol::BarRequest {
        v: request.v,
        dataset: request.dataset.clone(),
        symbol: request.symbol.clone(),
        stype_in: request.stype_in,
        resolution: request.resolution,
        gap_policy: request.gap_policy,
    }
}

#[cfg(feature = "databento-compat")]
fn history_request_for_subscription(request: &crate::protocol::BarRequest) -> HistoryRequest {
    // Only metadata and target-resolution semantics are used by the steady
    // forwarder. The bounded replay range is established before this request
    // is constructed, so these values are never sent upstream.
    let now = unix_seconds();
    HistoryRequest {
        v: request.v,
        dataset: request.dataset.clone(),
        symbol: request.symbol.clone(),
        stype_in: request.stype_in,
        resolution: request.resolution,
        gap_policy: request.gap_policy,
        from: now.saturating_sub(request.resolution.as_seconds()),
        to: now,
    }
}

fn response_payload(
    request: &HistoryRequest,
    bars: &[AggregatedBar],
    mappings: &[SymbolMapping],
) -> (Vec<ChartBar>, Vec<VolumePoint>, Vec<MetadataPoint>) {
    let mut chart = Vec::with_capacity(bars.len());
    let mut metadata = Vec::with_capacity(bars.len());
    let mut volumes = Vec::with_capacity(bars.len());

    let source_schema = request.resolution.source_schema();

    for bar in bars {
        let mapping = mappings
            .first()
            .expect("route_history_bars requires a resolved mapping");
        let meta = MetadataPoint {
            time: bar.time,
            meta: BarMetadata {
                dataset: request.dataset.clone(),
                requested_symbol: request.symbol.clone(),
                resolved_symbol: mapping.resolved_symbol.clone(),
                instrument_id: mapping.instrument_id,
                source_schema,
                synthetic: bar.synthetic,
            },
        };

        metadata.push(meta);
        if bar.whitespace {
            chart.push(ChartBar::Whitespace { time: bar.time });
            continue;
        }

        chart.push(ChartBar::Candlestick {
            time: bar.time,
            open: price_to_f64(bar.open),
            high: price_to_f64(bar.high),
            low: price_to_f64(bar.low),
            close: price_to_f64(bar.close),
        });

        if bar.volume > 0 {
            volumes.push(VolumePoint {
                time: bar.time,
                value: bar.volume as f64,
                color: None,
            });
        }
    }

    (chart, volumes, metadata)
}

fn request_id(headers: &HeaderMap) -> String {
    headers
        .get("X-Request-Id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("missing-request-id")
        .to_string()
}

fn gateway_error(error: GatewayError, headers: HeaderMap, _fallback: Option<String>) -> Response {
    let body = ErrorResponse {
        v: 1,
        request_id: request_id(&headers),
        error: ProviderError {
            code: error.error_body().code,
            message: error.error_body().message,
            retryable: error.error_body().retryable,
            details: error.error_body().details,
        },
    };
    (error.http_status(), Json(body)).into_response()
}

#[cfg(test)]
mod tests {
    use crate::protocol::{
        parse_client_command, parse_server_event, BarPageResponse, ErrorResponse, ResolveResponse,
    };
    use serde_json::Value;

    #[test]
    fn live_handoff_replays_from_lagging_historical_availability() {
        assert_eq!(
            super::live_handoff_boundaries(1_000, 2_000, Some(1_800), 60),
            (1_800, 1_740)
        );
        assert_eq!(
            super::live_handoff_boundaries(1_000, 2_000, Some(900), 60),
            (1_000, 1_000)
        );
        assert_eq!(
            super::live_handoff_boundaries(1_000, 2_000, None, 60),
            (2_000, 1_940)
        );
    }

    #[test]
    fn historical_resolution_uses_the_latest_available_window() {
        assert_eq!(
            super::historical_resolution_range(1_800, 2_000, Some(1_920), 60),
            (1_800, 1_920)
        );
        assert_eq!(
            super::historical_resolution_range(1_980, 2_000, Some(1_920), 60),
            (1_860, 1_920)
        );
        assert_eq!(
            super::historical_resolution_range(1_800, 2_000, None, 60),
            (1_800, 2_000)
        );
    }

    #[test]
    fn open_bars_requires_the_current_source_bucket() {
        assert_eq!(
            super::require_live_edge(1_800, 1_999, 2_000, 60).unwrap(),
            1_980
        );
        let stale = super::require_live_edge(1_800, 1_900, 2_000, 60).unwrap_err();
        assert_eq!(
            stale.error_body().code,
            crate::protocol::ProviderErrorCode::InvalidRange
        );
        let future = super::require_live_edge(1_800, 2_040, 2_000, 60).unwrap_err();
        assert_eq!(
            future.error_body().code,
            crate::protocol::ProviderErrorCode::InvalidRange
        );
        let empty_history = super::require_live_edge(1_980, 1_999, 2_000, 60).unwrap_err();
        assert_eq!(
            empty_history.error_body().code,
            crate::protocol::ProviderErrorCode::InvalidRange
        );
        assert_eq!(
            super::require_live_edge(1_800, 1_979, 2_000, 60).unwrap(),
            1_980
        );
        assert!(super::require_live_edge(1_800, 1_920, 2_000, 60).is_err());
    }

    #[test]
    fn history_metadata_requires_resolved_mapping() {
        let error = super::require_mapping(Vec::new()).unwrap_err();
        assert_eq!(
            error.error_body().code,
            crate::protocol::ProviderErrorCode::SymbolMappingFailed
        );
    }

    #[test]
    fn protocol_contract_invalid_ws_client_command_fixture() {
        let raw = include_str!(
            "../../../contracts/fixtures/websocket/invalid/unknown-command-field.json"
        );
        let fixture: Value = serde_json::from_str(raw).unwrap();
        let payload = fixture["payload"].to_string();
        assert!(parse_client_command(payload.as_bytes()).is_err());
    }

    #[test]
    fn protocol_contract_valid_ws_open_command_fixture() {
        let raw = include_str!("../../../contracts/fixtures/websocket/valid/open-bars.json");
        let fixture: Value = serde_json::from_str(raw).unwrap();
        let payload = fixture["payload"].to_string();
        assert!(parse_client_command(payload.as_bytes()).is_ok());
    }

    #[test]
    fn protocol_contract_accepts_every_valid_fixture() {
        let history: Value = serde_json::from_str(include_str!(
            "../../../contracts/fixtures/http/valid/history-response.json"
        ))
        .unwrap();
        let history: BarPageResponse = serde_json::from_value(history["payload"].clone()).unwrap();
        history.validate_time_range().unwrap();

        let resolve: Value = serde_json::from_str(include_str!(
            "../../../contracts/fixtures/http/valid/resolve-response.json"
        ))
        .unwrap();
        serde_json::from_value::<ResolveResponse>(resolve["payload"].clone()).unwrap();

        let error: Value = serde_json::from_str(include_str!(
            "../../../contracts/fixtures/http/valid/error-response.json"
        ))
        .unwrap();
        serde_json::from_value::<ErrorResponse>(error["payload"].clone()).unwrap();

        for raw in [
            include_str!("../../../contracts/fixtures/websocket/valid/bar.json"),
            include_str!("../../../contracts/fixtures/websocket/valid/cancelled.json"),
            include_str!(
                "../../../contracts/fixtures/websocket/valid/resolved-instrument-changed.json"
            ),
            include_str!("../../../contracts/fixtures/websocket/valid/snapshot.json"),
            include_str!("../../../contracts/fixtures/websocket/valid/subscribed.json"),
        ] {
            let fixture: Value = serde_json::from_str(raw).unwrap();
            let payload = fixture["payload"].to_string();
            parse_server_event(payload.as_bytes()).unwrap();
        }
    }

    #[test]
    fn protocol_contract_invalid_ws_bar_payload_fails() {
        let raw =
            include_str!("../../../contracts/fixtures/websocket/invalid/mismatched-volume.json");
        let fixture: Value = serde_json::from_str(raw).unwrap();
        let payload = fixture["payload"].to_string();
        assert!(parse_server_event(payload.as_bytes()).is_err());
    }

    #[test]
    fn protocol_contract_invalid_http_response_rejects_unsafe_time() {
        let raw = include_str!("../../../contracts/fixtures/http/invalid/unsafe-time.json");
        let fixture: Value = serde_json::from_str(raw).unwrap();
        let payload = fixture["payload"].clone();
        let response: BarPageResponse = serde_json::from_value(payload).unwrap();
        assert!(response.validate_time_range().is_err());
    }

    #[test]
    fn protocol_contract_invalid_http_response_rejects_credential_field() {
        let raw = include_str!("../../../contracts/fixtures/http/invalid/credential-field.json");
        let fixture: Value = serde_json::from_str(raw).unwrap();
        assert!(serde_json::from_value::<BarPageResponse>(fixture["payload"].clone()).is_err());
    }

    #[test]
    fn protocol_contract_unknown_ws_event_rejected() {
        let raw = include_str!("../../../contracts/fixtures/websocket/invalid/unknown-event.json");
        let fixture: Value = serde_json::from_str(raw).unwrap();
        let payload = fixture["payload"].to_string();
        assert!(parse_server_event(payload.as_bytes()).is_err());
    }

    #[cfg(feature = "databento-compat")]
    #[tokio::test]
    async fn replay_handoff_deduplicates_before_snapshot_and_waits_for_completion() {
        use crate::{
            live::databento::LiveEvent, normalization::NormalizedBaseBar, protocol::SourceSchema,
        };

        fn bar(time: i64, close: i64) -> NormalizedBaseBar {
            NormalizedBaseBar {
                time,
                dataset: "GLBX.MDP3".to_string(),
                instrument_id: 42,
                schema: SourceSchema::Ohlcv1m,
                open: close,
                high: close,
                low: close,
                close,
                volume: 1,
                synthetic: false,
            }
        }

        let (sender, mut receiver) = tokio::sync::mpsc::channel(4);
        sender.send(LiveEvent::Bar(bar(60, 11))).await.unwrap();
        sender.send(LiveEvent::ReplayCompleted).await.unwrap();
        let replay = super::wait_for_replay(&mut receiver).await.unwrap();
        let mut history = vec![bar(0, 10), bar(60, 9)];
        super::merge_base_bars(&mut history, replay);

        assert_eq!(history.len(), 2);
        assert_eq!(history[0].time, 0);
        assert_eq!(history[1].time, 60);
        assert_eq!(history[1].close, 11);
    }

    #[cfg(feature = "databento-compat")]
    #[tokio::test]
    async fn replay_handoff_reports_bounded_upstream_backpressure() {
        use crate::live::databento::{LiveBoundaryError, LiveEvent};
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        sender
            .send(LiveEvent::Failure(LiveBoundaryError::SlowConsumer))
            .await
            .unwrap();
        assert!(matches!(
            super::wait_for_replay(&mut receiver).await,
            Err(crate::protocol::ProviderErrorCode::SlowConsumer)
        ));
    }

    #[cfg(feature = "databento-compat")]
    #[tokio::test]
    async fn resume_replay_filters_overlap_replaces_equal_time_and_orders_live_boundary() {
        use crate::{
            live::databento::LiveEvent,
            normalization::NormalizedBaseBar,
            protocol::{GapPolicy, ProviderState, SourceSchema},
        };

        fn bar(time: i64, close: i64) -> NormalizedBaseBar {
            NormalizedBaseBar {
                time,
                dataset: "GLBX.MDP3".to_string(),
                instrument_id: 42,
                schema: SourceSchema::Ohlcv1m,
                open: close,
                high: close,
                low: close,
                close,
                volume: 1,
                synthetic: false,
            }
        }

        let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
        sender.send(LiveEvent::Bar(bar(0, 9))).await.unwrap(); // overlap: hidden
        sender.send(LiveEvent::Bar(bar(60, 10))).await.unwrap();
        sender.send(LiveEvent::Bar(bar(60, 11))).await.unwrap(); // replacement
        sender.send(LiveEvent::Bar(bar(120, 12))).await.unwrap();
        sender.send(LiveEvent::ReplayCompleted).await.unwrap();

        let replay = super::wait_for_resume_replay(&mut receiver, 60)
            .await
            .unwrap();
        assert_eq!(
            replay
                .iter()
                .map(|bar| (bar.time, bar.close))
                .collect::<Vec<_>>(),
            vec![(60, 11), (120, 12)]
        );

        let mut steady = super::SteadyAggregation::default();
        let emitted = replay
            .into_iter()
            .map(|bar| {
                steady
                    .update(bar, 60, GapPolicy::PreserveGaps)
                    .unwrap()
                    .time
            })
            .collect::<Vec<_>>();
        assert_eq!(emitted, vec![60, 120]);
        assert!(emitted.windows(2).all(|pair| pair[0] <= pair[1]));

        let lifecycle = [ProviderState::Replaying, ProviderState::Live];
        assert_eq!(lifecycle, [ProviderState::Replaying, ProviderState::Live]);
    }

    #[cfg(feature = "databento-compat")]
    #[test]
    fn steady_aggregation_revises_same_bucket_and_starts_next_bucket() {
        use crate::{
            normalization::NormalizedBaseBar,
            protocol::{GapPolicy, SourceSchema},
        };

        fn bar(time: i64, close: i64, volume: i64) -> NormalizedBaseBar {
            NormalizedBaseBar {
                time,
                dataset: "GLBX.MDP3".to_string(),
                instrument_id: 42,
                schema: SourceSchema::Ohlcv1m,
                open: close - 1,
                high: close + 1,
                low: close - 2,
                close,
                volume,
                synthetic: false,
            }
        }

        let mut steady = super::SteadyAggregation::default();
        let first = steady
            .update(bar(0, 10, 2), 300, GapPolicy::PreserveGaps)
            .unwrap();
        assert_eq!((first.time, first.close, first.volume), (0, 10, 2));

        let revised = steady
            .update(bar(0, 11, 3), 300, GapPolicy::PreserveGaps)
            .unwrap();
        assert_eq!((revised.time, revised.close, revised.volume), (0, 11, 3));

        let combined = steady
            .update(bar(60, 12, 5), 300, GapPolicy::PreserveGaps)
            .unwrap();
        assert_eq!(
            (
                combined.time,
                combined.open,
                combined.close,
                combined.volume
            ),
            (0, 10, 12, 8)
        );

        let next = steady
            .update(bar(300, 13, 7), 300, GapPolicy::PreserveGaps)
            .unwrap();
        assert_eq!((next.time, next.close, next.volume), (300, 13, 7));
    }
}
