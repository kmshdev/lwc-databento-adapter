use async_trait::async_trait;

use crate::error::GatewayError;
use crate::normalization::{
    align_range_start_seconds, assert_request_bounds, validate_and_normalize_bar,
    NormalizedBaseBar, RawBaseBar,
};
use crate::protocol::{
    DatasetMetadata, HistoryRequest, PublisherMetadata, ResolveRequest, SearchRequest,
    SearchResult, SymbolMapping, SymbolType,
};

/// Maximum records accepted from a single upstream historical request.
///
/// The browser contract is range-bounded, but a hard record cap also limits a
/// high-density request. The caller can request older data in pages.
#[cfg(feature = "databento-compat")]
const MAX_UPSTREAM_RECORDS: u64 = 50_000;

/// Official Databento Historical API implementation. It deliberately owns its
/// key only on the gateway process and creates a short-lived SDK client per
/// request, so cancelling the request future drops the in-flight decoder.
#[cfg(feature = "databento-compat")]
#[derive(Debug, Clone)]
pub struct DatabentoHistoricalSource {
    api_key: String,
    allowed_datasets: Vec<String>,
}

#[cfg(feature = "databento-compat")]
impl DatabentoHistoricalSource {
    pub fn new(api_key: String, allowed_datasets: Vec<String>) -> Result<Self, GatewayError> {
        if api_key.trim().is_empty() {
            return Err(GatewayError::invalid_request(
                "Databento API key is required",
            ));
        }
        if allowed_datasets.is_empty() {
            return Err(GatewayError::invalid_request(
                "at least one dataset is required",
            ));
        }
        Ok(Self {
            api_key,
            allowed_datasets,
        })
    }

    fn dataset_known(&self, dataset: &str) -> bool {
        self.allowed_datasets.iter().any(|entry| entry == dataset)
    }

    fn client(&self) -> Result<databento::HistoricalClient, GatewayError> {
        databento::HistoricalClient::builder()
            .key(&self.api_key)
            .and_then(|builder| builder.build())
            .map_err(|_| {
                GatewayError::protocol(
                    "Databento historical client configuration failed",
                    crate::protocol::ProviderErrorCode::UpstreamUnavailable,
                )
            })
    }

    fn validate_dataset_and_range(
        &self,
        dataset: &str,
        from: i64,
        to: i64,
    ) -> Result<(), GatewayError> {
        if !self.dataset_known(dataset) {
            return Err(GatewayError::invalid_request("unsupported dataset"));
        }
        assert_request_bounds(from, to)
    }
}

#[cfg(feature = "databento-compat")]
#[async_trait]
impl HistoricalSource for DatabentoHistoricalSource {
    async fn get_bars(
        &self,
        request: &HistoryRequest,
    ) -> Result<Vec<NormalizedBaseBar>, GatewayError> {
        use databento::dbn::OhlcvMsg;
        use std::num::NonZeroU64;

        if request.stype_in == SymbolType::Parent {
            return Err(GatewayError::unsupported_parent_series(
                "unsupported parent bars request",
            ));
        }
        self.validate_dataset_and_range(&request.dataset, request.from, request.to)?;

        let source_schema = request.resolution.source_schema();
        let aligned_from = align_range_start_seconds(request.from, &source_schema);
        if aligned_from >= request.to {
            return Ok(Vec::new());
        }
        let params = crate::compat::historical_range_params_with_limit(
            &request.dataset,
            &request.symbol,
            crate::compat::source_schema_to_dbn(source_schema),
            crate::compat::symbol_type_to_dbn(request.stype_in),
            aligned_from as u64,
            request.to as u64,
            NonZeroU64::new(MAX_UPSTREAM_RECORDS).expect("nonzero upstream record limit"),
        )
        .map_err(|_| GatewayError::invalid_range("invalid Databento historical range"))?;

        let mut client = self.client()?;
        let mut decoder = client.timeseries().get_range(&params).await.map_err(|_| {
            GatewayError::protocol(
                "Databento historical request failed",
                crate::protocol::ProviderErrorCode::UpstreamUnavailable,
            )
        })?;
        let mut bars = Vec::new();
        while let Some(record) = decoder.decode_record::<OhlcvMsg>().await.map_err(|_| {
            GatewayError::protocol(
                "Databento historical decode failed",
                crate::protocol::ProviderErrorCode::UpstreamUnavailable,
            )
        })? {
            let volume = i64::try_from(record.volume).map_err(|_| {
                GatewayError::protocol(
                    "Databento OHLCV volume exceeds gateway integer range",
                    crate::protocol::ProviderErrorCode::ProtocolError,
                )
            })?;
            let raw = RawBaseBar {
                // RawBaseBar also supports compile-time static fixtures. Its
                // dataset label is copied by normalization, so replace it with
                // the request dataset immediately below.
                dataset: "databento",
                instrument_id: i64::from(record.hd.instrument_id),
                schema: source_schema,
                start_ns: i64::try_from(record.hd.ts_event).map_err(|_| {
                    GatewayError::protocol(
                        "Databento OHLCV timestamp exceeds gateway integer range",
                        crate::protocol::ProviderErrorCode::ProtocolError,
                    )
                })?,
                open: record.open,
                high: record.high,
                low: record.low,
                close: record.close,
                volume,
            };
            let mut normalized = validate_and_normalize_bar(raw)?;
            normalized.dataset.clone_from(&request.dataset);
            bars.push(normalized);
        }
        bars.sort_by_key(|bar| bar.time);
        Ok(bars)
    }

    async fn resolve_symbols(
        &self,
        request: &ResolveRequest,
    ) -> Result<Vec<SymbolMapping>, GatewayError> {
        self.validate_dataset_and_range(&request.dataset, request.from, request.to)?;
        if request.symbols.is_empty() {
            return Err(GatewayError::invalid_request("symbols must not be empty"));
        }
        let params = crate::compat::resolve_params(
            &request.dataset,
            &request.symbols,
            crate::compat::symbol_type_to_dbn(request.stype_in),
            request.from,
            request.to,
        )
        .map_err(|_| GatewayError::invalid_range("invalid Databento resolution range"))?;
        let mut client = self.client()?;
        let resolution = client.symbology().resolve(&params).await.map_err(|_| {
            GatewayError::protocol(
                "Databento symbol resolution failed",
                crate::protocol::ProviderErrorCode::SymbolMappingFailed,
            )
        })?;
        let mut mappings = Vec::new();
        for (requested_symbol, intervals) in resolution.mappings {
            for interval in intervals {
                let instrument_id = interval.symbol.parse::<i64>().map_err(|_| {
                    GatewayError::protocol(
                        "Databento symbol resolution did not return an instrument id",
                        crate::protocol::ProviderErrorCode::SymbolMappingFailed,
                    )
                })?;
                mappings.push(SymbolMapping {
                    dataset: request.dataset.clone(),
                    requested_symbol: requested_symbol.clone(),
                    resolved_symbol: interval.symbol,
                    instrument_id,
                    effective_from: crate::compat::date_to_epoch_seconds(interval.start_date),
                    effective_to: Some(crate::compat::date_to_epoch_seconds(interval.end_date)),
                });
            }
        }
        if mappings.is_empty() {
            return Err(GatewayError::protocol(
                "Databento did not resolve the requested symbol",
                crate::protocol::ProviderErrorCode::SymbolNotFound,
            ));
        }
        mappings.sort_by_key(|mapping| (mapping.requested_symbol.clone(), mapping.effective_from));
        Ok(mappings)
    }

    async fn search_symbols(
        &self,
        request: &SearchRequest,
    ) -> Result<Vec<SearchResult>, GatewayError> {
        // Databento's Historical API resolves supplied symbols; it has no
        // unbounded text-search endpoint. Treat this bounded endpoint as an
        // exact query and return no result for an unresolved symbol.
        self.validate_dataset_and_range(&request.dataset, 1, 2)?;
        if request.query.trim().is_empty() {
            return Err(GatewayError::invalid_request("query must not be empty"));
        }
        let resolve = ResolveRequest {
            v: request.v,
            dataset: request.dataset.clone(),
            symbols: vec![request.query.clone()],
            stype_in: request.stype_in,
            from: 1_700_000_000,
            to: 1_700_000_001,
        };
        match self.resolve_symbols(&resolve).await {
            Ok(mappings) => Ok(mappings
                .into_iter()
                .map(|mapping| SearchResult {
                    dataset: mapping.dataset,
                    symbol: mapping.requested_symbol,
                    stype_in: request.stype_in,
                    description: Some(format!("instrument_id {}", mapping.instrument_id)),
                })
                .collect()),
            Err(GatewayError::Protocol {
                code: crate::protocol::ProviderErrorCode::SymbolNotFound,
                ..
            }) => Ok(Vec::new()),
            Err(error) => Err(error),
        }
    }

    async fn dataset_metadata(&self, dataset: &str) -> Result<DatasetMetadata, GatewayError> {
        if !self.dataset_known(dataset) {
            return Err(GatewayError::invalid_request("unsupported dataset"));
        }
        let mut client = self.client()?;
        let schemas = client.metadata().list_schemas(dataset).await.map_err(|_| {
            GatewayError::protocol(
                "Databento dataset metadata request failed",
                crate::protocol::ProviderErrorCode::UpstreamUnavailable,
            )
        })?;
        let available = client
            .metadata()
            .get_dataset_range(dataset)
            .await
            .map_err(|_| {
                GatewayError::protocol(
                    "Databento dataset range request failed",
                    crate::protocol::ProviderErrorCode::UpstreamUnavailable,
                )
            })?;
        let publishers = client.metadata().list_publishers().await.map_err(|_| {
            GatewayError::protocol(
                "Databento publisher metadata request failed",
                crate::protocol::ProviderErrorCode::UpstreamUnavailable,
            )
        })?;
        Ok(DatasetMetadata {
            dataset: dataset.to_string(),
            schemas: schemas
                .into_iter()
                .map(|schema| schema.as_str().to_string())
                .collect(),
            publishers: publishers
                .into_iter()
                .filter(|publisher| publisher.dataset == dataset)
                .map(|publisher| PublisherMetadata {
                    publisher_id: i64::from(publisher.publisher_id),
                    name: publisher.description,
                    venue: publisher.venue,
                })
                .collect(),
            available_from: Some(available.start.unix_timestamp()),
            available_to: Some(available.end.unix_timestamp()),
        })
    }
}

#[async_trait]
pub trait HistoricalSource: Send + Sync {
    async fn get_bars(
        &self,
        request: &HistoryRequest,
    ) -> Result<Vec<NormalizedBaseBar>, GatewayError>;
    async fn resolve_symbols(
        &self,
        request: &ResolveRequest,
    ) -> Result<Vec<SymbolMapping>, GatewayError>;
    async fn search_symbols(
        &self,
        request: &SearchRequest,
    ) -> Result<Vec<SearchResult>, GatewayError>;
    async fn dataset_metadata(&self, dataset: &str) -> Result<DatasetMetadata, GatewayError>;
}

#[derive(Debug, Clone)]
pub struct HistoricalSourceConfig {
    pub default_request_gap_seconds: i64,
}

#[derive(Debug, Clone)]
pub struct FakeHistorySource {
    bars: Vec<RawBaseBar>,
    mappings: Vec<SymbolMapping>,
    datasets: Vec<String>,
}

impl FakeHistorySource {
    pub fn from_fixtures(
        bars: Vec<RawBaseBar>,
        mappings: Vec<SymbolMapping>,
        datasets: Vec<String>,
    ) -> Self {
        Self {
            bars,
            mappings,
            datasets,
        }
    }

    pub fn empty() -> Self {
        Self::from_fixtures(
            Vec::new(),
            Vec::new(),
            vec!["GLBX.MDP3".to_string(), "XNAS.ITCH".to_string()],
        )
    }

    pub fn demo(now_seconds: i64) -> Self {
        let end = now_seconds - now_seconds.rem_euclid(60);
        let start = end - 7 * 86_400;
        let mut bars = Vec::new();
        for (index, time) in (start..end).step_by(60).enumerate() {
            let base = 5_000_000_000_000_i64 + (index as i64 % 500) * 10_000_000;
            bars.push(RawBaseBar {
                dataset: "GLBX.MDP3",
                instrument_id: 123,
                schema: crate::protocol::SourceSchema::Ohlcv1m,
                start_ns: time.saturating_mul(1_000_000_000),
                open: base,
                high: base + 2_000_000_000,
                low: base - 1_000_000_000,
                close: base + 1_000_000_000,
                volume: 100 + index as i64 % 50,
            });
        }
        let mappings = vec![
            SymbolMapping {
                dataset: "GLBX.MDP3".to_string(),
                requested_symbol: "ES.c.0".to_string(),
                resolved_symbol: "ESZ4".to_string(),
                instrument_id: 123,
                effective_from: start,
                effective_to: None,
            },
            SymbolMapping {
                dataset: "GLBX.MDP3".to_string(),
                requested_symbol: "ES.FUT.ESZ4".to_string(),
                resolved_symbol: "ESZ4".to_string(),
                instrument_id: 123,
                effective_from: start,
                effective_to: None,
            },
            SymbolMapping {
                dataset: "GLBX.MDP3".to_string(),
                requested_symbol: "ES.FUT.ESH5".to_string(),
                resolved_symbol: "ESH5".to_string(),
                instrument_id: 124,
                effective_from: start,
                effective_to: None,
            },
        ];
        Self::from_fixtures(bars, mappings, vec!["GLBX.MDP3".to_string()])
    }

    fn dataset_known(&self, dataset: &str) -> bool {
        self.datasets.iter().any(|entry| entry == dataset)
    }

    fn first_mapping_for_symbol(&self, request: &HistoryRequest) -> Option<&SymbolMapping> {
        self.mappings
            .iter()
            .filter(|entry| {
                entry.dataset == request.dataset
                    && matches_symbol_type(entry.requested_symbol.as_str(), request)
                    && entry.effective_from <= request.to
                    && entry.effective_to.unwrap_or(i64::MAX).saturating_add(1) >= request.from
            })
            .min_by_key(|entry| entry.effective_from)
    }
}

#[async_trait]
impl HistoricalSource for FakeHistorySource {
    async fn get_bars(
        &self,
        request: &HistoryRequest,
    ) -> Result<Vec<NormalizedBaseBar>, GatewayError> {
        if !self.dataset_known(&request.dataset) {
            return Err(GatewayError::invalid_request("unsupported dataset"));
        }
        if request.stype_in == SymbolType::Parent {
            return Err(GatewayError::unsupported_parent_series(
                "unsupported parent bars request",
            ));
        }

        assert_request_bounds(request.from, request.to)?;

        let source_schema = request.resolution.source_schema();
        let aligned_from = align_range_start_seconds(request.from, &source_schema);
        if aligned_from >= request.to {
            return Ok(Vec::new());
        }

        let mapped = self.first_mapping_for_symbol(request);
        let target_instrument_id = match request.stype_in {
            SymbolType::InstrumentId => request
                .symbol
                .parse::<i64>()
                .map_err(|_| GatewayError::invalid_request("invalid instrument id"))?,
            SymbolType::RawSymbol | SymbolType::Continuous => mapped
                .map(|entry| entry.instrument_id)
                .ok_or_else(|| GatewayError::invalid_request("unknown symbol"))?,
            SymbolType::Parent => unreachable!("parent rejected above"),
        };

        let mut out = Vec::new();
        for bar in &self.bars {
            if bar.dataset != request.dataset {
                continue;
            }
            if bar.schema != source_schema {
                continue;
            }
            if bar.instrument_id != target_instrument_id {
                continue;
            }

            let normalized = validate_and_normalize_bar(bar.clone())?;
            if normalized.time < aligned_from || normalized.time >= request.to {
                continue;
            }
            out.push(normalized);
        }

        out.sort_by_key(|value| value.time);
        Ok(out)
    }

    async fn resolve_symbols(
        &self,
        request: &ResolveRequest,
    ) -> Result<Vec<SymbolMapping>, GatewayError> {
        if !self.dataset_known(&request.dataset) {
            return Err(GatewayError::invalid_request("unsupported dataset"));
        }
        if request.symbols.is_empty() {
            return Err(GatewayError::invalid_request("symbols must not be empty"));
        }

        let mut out = Vec::new();
        for symbol in &request.symbols {
            match request.stype_in {
                SymbolType::Parent => {
                    out.extend(
                        self.mappings
                            .iter()
                            .filter(|entry| {
                                entry.dataset == request.dataset
                                    && entry.requested_symbol.starts_with(symbol)
                            })
                            .cloned(),
                    );
                }
                _ => {
                    if let Some(found) = self.mappings.iter().find(|entry| {
                        entry.dataset == request.dataset && entry.requested_symbol == *symbol
                    }) {
                        out.push(found.clone());
                        continue;
                    }
                    if request.stype_in == SymbolType::InstrumentId {
                        let instrument_id = symbol
                            .parse::<i64>()
                            .map_err(|_| GatewayError::invalid_request("invalid instrument_id"))?;
                        if let Some(found) = self.mappings.iter().find(|entry| {
                            entry.dataset == request.dataset && entry.instrument_id == instrument_id
                        }) {
                            out.push(found.clone());
                        }
                    }
                }
            }
        }

        if out.is_empty() {
            return Err(GatewayError::invalid_request("unknown symbol"));
        }

        Ok(out)
    }

    async fn search_symbols(
        &self,
        request: &SearchRequest,
    ) -> Result<Vec<SearchResult>, GatewayError> {
        if !self.dataset_known(&request.dataset) {
            return Err(GatewayError::invalid_request("unsupported dataset"));
        }

        let q = request.query.to_lowercase();
        let mut out = Vec::new();
        for mapping in &self.mappings {
            if mapping.dataset != request.dataset {
                continue;
            }
            if request.stype_in != SymbolType::Parent && mapping.requested_symbol != request.query {
                continue;
            }
            if !mapping.requested_symbol.to_lowercase().contains(&q) {
                continue;
            }

            out.push(SearchResult {
                dataset: mapping.dataset.clone(),
                symbol: mapping.requested_symbol.clone(),
                stype_in: request.stype_in,
                description: Some(format!(
                    "{} {} -> {} ({})",
                    mapping.dataset,
                    mapping.requested_symbol,
                    mapping.resolved_symbol,
                    mapping.instrument_id
                )),
            });
        }
        out.sort_by(|left, right| left.symbol.cmp(&right.symbol));
        Ok(out)
    }

    async fn dataset_metadata(&self, dataset: &str) -> Result<DatasetMetadata, GatewayError> {
        if !self.dataset_known(dataset) {
            return Err(GatewayError::invalid_request("unsupported dataset"));
        }

        let schemas: Vec<String> = match dataset {
            "GLBX.MDP3" => vec![
                "ohlcv-1s".to_string(),
                "ohlcv-1m".to_string(),
                "ohlcv-1h".to_string(),
                "ohlcv-1d".to_string(),
            ],
            "XNAS.ITCH" => vec!["ohlcv-1m".to_string(), "ohlcv-1d".to_string()],
            _ => vec!["ohlcv-1m".to_string()],
        };

        Ok(DatasetMetadata {
            dataset: dataset.to_string(),
            schemas,
            publishers: vec![PublisherMetadata {
                publisher_id: 1,
                name: "local-fake".to_string(),
                venue: "loopback".to_string(),
            }],
            available_from: Some(1_700_000_000),
            available_to: Some(1_800_000_000),
        })
    }
}

fn matches_symbol_type(symbol: &str, request: &HistoryRequest) -> bool {
    match request.stype_in {
        SymbolType::InstrumentId => symbol == request.symbol,
        SymbolType::RawSymbol => symbol == request.symbol,
        SymbolType::Continuous => true,
        SymbolType::Parent => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalization::RawBaseBar;
    use crate::protocol::{GapPolicy, Resolution, SourceSchema, SymbolType};

    fn fixture() -> FakeHistorySource {
        FakeHistorySource::from_fixtures(
            vec![
                RawBaseBar {
                    dataset: "GLBX.MDP3",
                    instrument_id: 123,
                    schema: SourceSchema::Ohlcv1m,
                    start_ns: 1_700_000_000_000_000_000,
                    open: 100_000_000_000,
                    high: 102_000_000_000,
                    low: 99_000_000_000,
                    close: 101_000_000_000,
                    volume: 10,
                },
                RawBaseBar {
                    dataset: "GLBX.MDP3",
                    instrument_id: 123,
                    schema: SourceSchema::Ohlcv1m,
                    start_ns: 1_700_000_060_000_000_000,
                    open: 101_000_000_000,
                    high: 103_000_000_000,
                    low: 99_500_000_000,
                    close: 102_000_000_000,
                    volume: 20,
                },
                RawBaseBar {
                    dataset: "GLBX.MDP3",
                    instrument_id: 123,
                    schema: SourceSchema::Ohlcv1m,
                    start_ns: 1_700_000_120_000_000_000,
                    open: 102_000_000_000,
                    high: 104_000_000_000,
                    low: 100_000_000_000,
                    close: 103_000_000_000,
                    volume: 30,
                },
            ],
            vec![
                SymbolMapping {
                    dataset: "GLBX.MDP3".to_string(),
                    requested_symbol: "ESZ4".to_string(),
                    resolved_symbol: "ESZ4".to_string(),
                    instrument_id: 123,
                    effective_from: 1_600_000_000,
                    effective_to: None,
                },
                SymbolMapping {
                    dataset: "GLBX.MDP3".to_string(),
                    requested_symbol: "ESZ4-continuous".to_string(),
                    resolved_symbol: "ESZ4".to_string(),
                    instrument_id: 123,
                    effective_from: 1_600_000_000,
                    effective_to: None,
                },
            ],
            vec!["GLBX.MDP3".to_string(), "XNAS.ITCH".to_string()],
        )
    }

    #[tokio::test]
    async fn get_bars_rejects_parent_symbol() {
        let source = fixture();
        let request = HistoryRequest {
            v: 1,
            dataset: "GLBX.MDP3".to_string(),
            symbol: "ESZ4".to_string(),
            stype_in: SymbolType::Parent,
            resolution: Resolution::OneMinute,
            from: 1_700_000_000,
            to: 1_700_010_000,
            gap_policy: Some(GapPolicy::PreserveGaps),
        };

        let error = source.get_bars(&request).await.unwrap_err();
        assert_eq!(
            error.error_body().code,
            crate::protocol::ProviderErrorCode::UnsupportedParentSeries
        );
    }

    #[tokio::test]
    async fn get_bars_skips_incomplete_leading_source_interval() {
        let source = fixture();
        let request = HistoryRequest {
            v: 1,
            dataset: "GLBX.MDP3".to_string(),
            symbol: "ESZ4".to_string(),
            stype_in: SymbolType::RawSymbol,
            resolution: Resolution::FiveMinute,
            from: 1_700_000_002,
            to: 1_700_001_000,
            gap_policy: Some(GapPolicy::PreserveGaps),
        };

        let values = source.get_bars(&request).await.unwrap();
        assert!(values.iter().all(|value| value.time >= 1_700_000_040));
        assert!(
            values.len() > 1,
            "custom aggregation must retain all source components"
        );
    }

    #[tokio::test]
    async fn resolve_parent_returns_child_candidates() {
        let source = fixture();
        let request = ResolveRequest {
            v: 1,
            dataset: "GLBX.MDP3".to_string(),
            symbols: vec!["ES".to_string()],
            stype_in: SymbolType::Parent,
            from: 0,
            to: 0,
        };

        let mappings = source.resolve_symbols(&request).await.unwrap();
        assert!(!mappings.is_empty());
        assert!(mappings
            .iter()
            .all(|entry| entry.requested_symbol.starts_with("ES")));
    }

    #[cfg(feature = "databento-compat")]
    #[test]
    fn real_source_requires_nonempty_key_without_contacting_upstream() {
        let error = DatabentoHistoricalSource::new(" ".to_string(), vec!["GLBX.MDP3".to_string()])
            .expect_err("blank key must be rejected before any client is created");
        assert_eq!(
            error.error_body().code,
            crate::protocol::ProviderErrorCode::InvalidRequest
        );
    }
}
