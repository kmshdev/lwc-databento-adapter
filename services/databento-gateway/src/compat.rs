#[cfg(feature = "databento-compat")]
use databento::{
    dbn::{SType, Schema, SystemCode, SystemMsg},
    historical::{symbology::ResolveParams, timeseries::GetRangeParams, DateRange, DateTimeRange},
    live::Subscription,
    HistoricalClient, LiveClient,
};

#[cfg(feature = "databento-compat")]
use std::num::NonZeroU64;

#[cfg(feature = "databento-compat")]
use time::{Date, Time};

pub fn has_databento_compat() -> bool {
    cfg!(feature = "databento-compat")
}

#[cfg(feature = "databento-compat")]
pub fn historical_range_params(
    dataset: &str,
    symbol: &str,
    schema: Schema,
    stype_in: SType,
    from_seconds: u64,
    to_seconds: u64,
) -> databento::Result<GetRangeParams> {
    let range = DateTimeRange::try_from((
        from_seconds.saturating_mul(1_000_000_000),
        to_seconds.saturating_mul(1_000_000_000),
    ))?;
    Ok(GetRangeParams::builder()
        .dataset(dataset)
        .symbols(symbol)
        .schema(schema)
        .stype_in(stype_in)
        .date_time_range(range)
        .build())
}

#[cfg(feature = "databento-compat")]
pub fn historical_range_params_with_limit(
    dataset: &str,
    symbol: &str,
    schema: Schema,
    stype_in: SType,
    from_seconds: u64,
    to_seconds: u64,
    limit: NonZeroU64,
) -> databento::Result<GetRangeParams> {
    let range = DateTimeRange::try_from((
        from_seconds.saturating_mul(1_000_000_000),
        to_seconds.saturating_mul(1_000_000_000),
    ))?;
    Ok(GetRangeParams::builder()
        .dataset(dataset)
        .symbols(symbol)
        .schema(schema)
        .stype_in(stype_in)
        .date_time_range(range)
        .limit(limit)
        .build())
}

#[cfg(feature = "databento-compat")]
pub fn symbol_type_to_dbn(symbol_type: crate::protocol::SymbolType) -> SType {
    match symbol_type {
        crate::protocol::SymbolType::RawSymbol => SType::RawSymbol,
        crate::protocol::SymbolType::InstrumentId => SType::InstrumentId,
        crate::protocol::SymbolType::Parent => SType::Parent,
        crate::protocol::SymbolType::Continuous => SType::Continuous,
    }
}

#[cfg(feature = "databento-compat")]
pub fn source_schema_to_dbn(schema: crate::protocol::SourceSchema) -> Schema {
    match schema {
        crate::protocol::SourceSchema::Ohlcv1s => Schema::Ohlcv1S,
        crate::protocol::SourceSchema::Ohlcv1m => Schema::Ohlcv1M,
        crate::protocol::SourceSchema::Ohlcv1h => Schema::Ohlcv1H,
        crate::protocol::SourceSchema::Ohlcv1d => Schema::Ohlcv1D,
    }
}

#[cfg(feature = "databento-compat")]
pub fn resolve_params(
    dataset: &str,
    symbols: &[String],
    stype_in: SType,
    from_seconds: i64,
    to_seconds: i64,
) -> databento::Result<ResolveParams> {
    let range = DateTimeRange::try_from((
        u64::try_from(from_seconds)
            .unwrap_or_default()
            .saturating_mul(1_000_000_000),
        u64::try_from(to_seconds)
            .unwrap_or_default()
            .saturating_mul(1_000_000_000),
    ))?;
    Ok(ResolveParams::builder()
        .dataset(dataset)
        .symbols(symbols.to_vec())
        .stype_in(stype_in)
        .stype_out(SType::InstrumentId)
        .date_range(DateRange::from(range))
        .build())
}

#[cfg(feature = "databento-compat")]
pub fn date_to_epoch_seconds(date: Date) -> i64 {
    date.with_time(Time::MIDNIGHT).assume_utc().unix_timestamp()
}

#[cfg(feature = "databento-compat")]
pub fn replay_subscription(
    symbol: &str,
    schema: Schema,
    stype_in: SType,
    range: &DateTimeRange,
) -> Subscription {
    Subscription::builder()
        .symbols(symbol)
        .schema(schema)
        .stype_in(stype_in)
        .start(range.start)
        .build()
}

#[cfg(feature = "databento-compat")]
pub async fn stream_historical_range(
    client: &mut HistoricalClient,
    params: &GetRangeParams,
) -> databento::Result<bool> {
    let mut decoder = client.timeseries().get_range(params).await?;
    Ok(decoder.decode_record_ref().await?.is_some())
}

#[cfg(feature = "databento-compat")]
pub async fn run_live_replay_probe(
    client: &mut LiveClient,
    subscription: Subscription,
) -> databento::Result<bool> {
    client.subscribe(subscription).await?;
    client.start().await?;
    let replay_completed = match client.next_record().await? {
        Some(record) => record
            .get::<SystemMsg>()
            .is_some_and(|message| message.code == SystemCode::ReplayCompleted as u8),
        None => false,
    };
    client.close().await?;
    Ok(replay_completed)
}

#[cfg(all(test, feature = "databento-compat"))]
mod tests {
    use super::*;

    #[test]
    fn pinned_historical_and_live_builders_compile() {
        let params = historical_range_params(
            "GLBX.MDP3",
            "ES.c.0",
            Schema::Ohlcv1M,
            SType::Continuous,
            1_700_000_000,
            1_700_000_600,
        )
        .unwrap();
        let range = params.date_time_range.clone();
        let subscription =
            replay_subscription("ES.c.0", Schema::Ohlcv1M, SType::Continuous, &range);
        assert_eq!(subscription.start, Some(range.start));
    }
}
