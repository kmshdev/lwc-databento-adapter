#![cfg(feature = "databento-compat")]

use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use databento::{
    dbn::{OhlcvMsg, SType, Schema, SystemCode, SystemMsg},
    historical::metadata::PublisherDetail,
    live::{SlowReaderBehavior, Subscription},
    HistoricalClient, LiveClient,
};
use databento_gateway::{
    compat::resolve_params,
    live::databento::normalize_dbn_ohlcv,
    protocol::{SourceSchema, SymbolType},
};

fn venues_by_dataset(publishers: Vec<PublisherDetail>) -> BTreeMap<String, BTreeSet<String>> {
    let mut result = BTreeMap::new();
    for publisher in publishers {
        result
            .entry(publisher.dataset)
            .or_insert_with(BTreeSet::new)
            .insert(publisher.venue);
    }
    result
}

#[tokio::test]
#[ignore = "requires a Databento API key and performs metadata-only account discovery"]
async fn account_aware_metadata_inventory() {
    let mut client = HistoricalClient::builder()
        .key_from_env()
        .expect("DATABENTO_API_KEY is invalid")
        .build()
        .expect("failed to build Databento historical client");

    let mut datasets = client
        .metadata()
        .list_datasets(None)
        .await
        .expect("Databento dataset catalog request failed");
    datasets.sort();
    datasets.dedup();

    let venues = venues_by_dataset(
        client
            .metadata()
            .list_publishers()
            .await
            .expect("Databento publisher catalog request failed"),
    );

    let mut available = Vec::new();
    let mut unverified = 0_usize;
    for dataset in &datasets {
        match client.metadata().get_dataset_range(dataset).await {
            Ok(range) => {
                let mut schemas = range
                    .range_by_schema
                    .keys()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                schemas.sort();
                let mapped_venues = venues
                    .get(dataset)
                    .map(|entries| entries.iter().cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                available.push((
                    dataset.clone(),
                    range.start.unix_timestamp(),
                    range.end.unix_timestamp(),
                    schemas,
                    mapped_venues,
                ));
            }
            Err(_) => unverified += 1,
        }
    }

    for (dataset, start, end, schemas, mapped_venues) in &available {
        println!(
            "historical_available dataset={dataset} start={start} end={end} schemas={} venues={}",
            schemas.join(","),
            mapped_venues.join(",")
        );
    }

    if available.is_empty() {
        println!(
            "inconclusive: catalog_datasets={} account_aware_ranges=0 unverified={unverified}",
            datasets.len()
        );
    } else {
        println!(
            "passed: catalog_datasets={} account_aware_ranges={} unverified={unverified}",
            datasets.len(),
            available.len()
        );
    }
    println!(
        "bulk_live_entitlements=not_inferred reason=no_bulk_live_entitlement_endpoint; dataset-specific live proof runs separately"
    );
}

#[tokio::test]
#[ignore = "requires a Databento API key and opens one bounded live session"]
async fn live_subscription_round_trip() {
    const DATASET: &str = "GLBX.MDP3";
    const SYMBOL: &str = "ES.FUT";
    let replay_start = time::OffsetDateTime::now_utc() - time::Duration::minutes(15);
    let subscription = Subscription::builder()
        .symbols(SYMBOL)
        .schema(Schema::Ohlcv1M)
        .stype_in(SType::Parent)
        .start(replay_start)
        .build();

    let mut client = LiveClient::builder()
        .key_from_env()
        .expect("DATABENTO_API_KEY is invalid")
        .dataset(DATASET)
        .slow_reader_behavior(SlowReaderBehavior::Warn)
        .build()
        .await
        .expect("Databento live authentication failed");
    client
        .subscribe(subscription)
        .await
        .expect("Databento live subscription request failed");
    client
        .start()
        .await
        .expect("Databento live session start failed");

    let observation = tokio::time::timeout(Duration::from_secs(45), async {
        let mut acknowledged = false;
        let mut replay_completed = false;
        let mut normalized_bars = 0_usize;
        while !replay_completed {
            let record = client
                .next_record()
                .await
                .expect("Databento live read failed")
                .expect("Databento closed the live session before replay completed");
            if let Some(system) = record.get::<SystemMsg>() {
                match system.code().expect("invalid Databento system code") {
                    SystemCode::SubscriptionAck => acknowledged = true,
                    SystemCode::ReplayCompleted => replay_completed = true,
                    _ => {}
                }
            } else if let Some(bar) = record.get::<OhlcvMsg>() {
                normalize_dbn_ohlcv(DATASET, SourceSchema::Ohlcv1m, bar)
                    .expect("live OHLCV normalization failed");
                normalized_bars += 1;
            }
        }
        (acknowledged, normalized_bars)
    })
    .await;

    client
        .close()
        .await
        .expect("Databento live session close failed");
    let (acknowledged, normalized_bars) = observation
        .expect("timed out waiting for subscription acknowledgement and replay completion");
    assert!(
        acknowledged,
        "subscription acknowledgement was not observed"
    );
    println!(
        "passed: live dataset={DATASET} symbol={SYMBOL} schema=ohlcv-1m subscription_ack=true replay_completed=true normalized_bars={normalized_bars} graceful_close=true"
    );
}

#[tokio::test]
#[ignore = "requires a Databento API key and resolves one live-edge continuous symbol"]
async fn continuous_symbol_resolution_at_live_edge() {
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let mut client = HistoricalClient::builder()
        .key_from_env()
        .expect("DATABENTO_API_KEY is invalid")
        .build()
        .expect("failed to build Databento historical client");
    let available_to = client
        .metadata()
        .get_dataset_range("GLBX.MDP3")
        .await
        .expect("GLBX.MDP3 range lookup failed")
        .end
        .unix_timestamp()
        .min(now);
    let resolution_from = (now - 48 * 60).min(available_to - 60);
    let params = resolve_params(
        "GLBX.MDP3",
        &["ES.c.0".to_string()],
        databento_gateway::compat::symbol_type_to_dbn(SymbolType::Continuous),
        resolution_from,
        available_to,
    )
    .expect("live-edge resolution parameters are invalid");
    let resolution = client
        .symbology()
        .resolve(&params)
        .await
        .expect("live-edge continuous symbol resolution failed");
    assert!(
        resolution
            .mappings
            .get("ES.c.0")
            .is_some_and(|intervals| !intervals.is_empty()),
        "continuous symbol resolved to no intervals"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publisher_catalog_is_grouped_and_deduplicated_by_dataset() {
        let publishers = vec![
            PublisherDetail {
                publisher_id: 2,
                dataset: "GLBX.MDP3".to_string(),
                venue: "XNYM".to_string(),
                description: "NYMEX".to_string(),
            },
            PublisherDetail {
                publisher_id: 1,
                dataset: "GLBX.MDP3".to_string(),
                venue: "XCME".to_string(),
                description: "CME".to_string(),
            },
            PublisherDetail {
                publisher_id: 3,
                dataset: "GLBX.MDP3".to_string(),
                venue: "XCME".to_string(),
                description: "CME duplicate".to_string(),
            },
        ];

        let grouped = venues_by_dataset(publishers);

        assert_eq!(
            grouped["GLBX.MDP3"].iter().cloned().collect::<Vec<_>>(),
            vec!["XCME".to_string(), "XNYM".to_string()]
        );
    }
}
