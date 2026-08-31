use std::collections::BTreeMap;

use crate::normalization::NormalizedBaseBar;
use crate::protocol::GapPolicy;

#[derive(Debug, Clone)]
pub struct AggregatedBar {
    pub time: i64,
    pub open: i64,
    pub high: i64,
    pub low: i64,
    pub close: i64,
    pub volume: i64,
    pub synthetic: bool,
    pub whitespace: bool,
}

#[derive(Debug)]
pub struct AggregationConfig {
    pub target_resolution_sec: i64,
    pub gap_policy: GapPolicy,
}

impl AggregationConfig {
    pub fn new(target_resolution_sec: i64, gap_policy: GapPolicy) -> Self {
        Self {
            target_resolution_sec,
            gap_policy,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregationError {
    EmptyRange,
    UnsupportedBucket,
    Overflow,
}

pub fn bucket_start(time: i64, bucket_sec: i64) -> i64 {
    time - time.rem_euclid(bucket_sec)
}

pub fn aggregate(
    components: &[NormalizedBaseBar],
    from: i64,
    to: i64,
    config: &AggregationConfig,
) -> Result<Vec<AggregatedBar>, AggregationError> {
    if config.target_resolution_sec <= 0 {
        return Err(AggregationError::UnsupportedBucket);
    }
    if from >= to {
        return Err(AggregationError::EmptyRange);
    }

    let mut buckets: BTreeMap<i64, BTreeMap<i64, &NormalizedBaseBar>> = BTreeMap::new();
    for component in components {
        if component.time < from || component.time >= to {
            continue;
        }
        let bucket = bucket_start(component.time, config.target_resolution_sec);
        let entry = buckets.entry(bucket).or_default();
        // Duplicate components for same source interval replace earlier records.
        entry.insert(component.time, component);
    }

    let mut out = Vec::new();
    let mut cursor = bucket_start(from, config.target_resolution_sec);
    if cursor < from {
        cursor += config.target_resolution_sec;
    }
    let mut previous_close = None;

    while cursor < to {
        if let Some(components) = buckets.get(&cursor) {
            if components.is_empty() {
                fill_gap(&mut out, cursor, &mut previous_close, &config.gap_policy);
            } else {
                let mut times = components.keys().copied().collect::<Vec<_>>();
                times.sort_unstable();
                let first = components
                    .get(&times[0])
                    .copied()
                    .expect("component map cannot be empty");
                let last = components
                    .get(times.last().expect("component map cannot be empty"))
                    .copied()
                    .expect("component map cannot be empty");

                let open = first.open;
                let mut high = first.high;
                let mut low = first.low;
                let close = last.close;
                let mut volume = 0_i64;
                for time in times {
                    let component = components.get(&time).expect("sorted key validated");
                    if component.high > high {
                        high = component.high;
                    }
                    if component.low < low {
                        low = component.low;
                    }
                    match volume.checked_add(component.volume) {
                        Some(sum) => volume = sum,
                        None => return Err(AggregationError::Overflow),
                    }
                }

                previous_close = Some(close);
                out.push(AggregatedBar {
                    time: cursor,
                    open,
                    high,
                    low,
                    close,
                    volume,
                    synthetic: false,
                    whitespace: false,
                });
            }
        } else {
            fill_gap(&mut out, cursor, &mut previous_close, &config.gap_policy);
        }
        cursor += config.target_resolution_sec;
    }

    Ok(out)
}

fn fill_gap(
    out: &mut Vec<AggregatedBar>,
    cursor: i64,
    previous_close: &mut Option<i64>,
    policy: &GapPolicy,
) {
    match policy {
        GapPolicy::PreserveGaps => {}
        GapPolicy::Whitespace => out.push(AggregatedBar {
            time: cursor,
            open: 0,
            high: 0,
            low: 0,
            close: 0,
            volume: 0,
            synthetic: true,
            whitespace: true,
        }),
        GapPolicy::CarryForward => {
            if let Some(close) = *previous_close {
                out.push(AggregatedBar {
                    time: cursor,
                    open: close,
                    high: close,
                    low: close,
                    close,
                    volume: 0,
                    synthetic: true,
                    whitespace: false,
                });
                *previous_close = Some(close);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::SourceSchema;

    fn bar(time: i64, close: i64, volume: i64) -> NormalizedBaseBar {
        NormalizedBaseBar {
            time,
            dataset: "GLBX.MDP3".to_string(),
            instrument_id: 7,
            schema: SourceSchema::Ohlcv1m,
            open: close - 1,
            high: close + 2,
            low: close - 2,
            close,
            volume,
            synthetic: false,
        }
    }

    #[test]
    fn aggregates_all_components_and_replaces_duplicates() {
        let values = vec![
            bar(300, 101, 10),
            bar(360, 102, 20),
            bar(360, 103, 30),
            bar(420, 104, 40),
        ];
        let result = aggregate(
            &values,
            300,
            600,
            &AggregationConfig::new(300, GapPolicy::PreserveGaps),
        )
        .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].open, 100);
        assert_eq!(result[0].close, 104);
        assert_eq!(result[0].volume, 80);
    }

    #[test]
    fn skips_incomplete_leading_target_bucket() {
        let result = aggregate(
            &[bar(360, 102, 20), bar(600, 103, 30)],
            360,
            900,
            &AggregationConfig::new(300, GapPolicy::PreserveGaps),
        )
        .unwrap();
        assert_eq!(
            result.iter().map(|value| value.time).collect::<Vec<_>>(),
            vec![600]
        );
    }

    #[test]
    fn distinguishes_whitespace_and_carry_forward_gaps() {
        let values = [bar(0, 101, 10), bar(120, 103, 20)];
        let whitespace = aggregate(
            &values,
            0,
            180,
            &AggregationConfig::new(60, GapPolicy::Whitespace),
        )
        .unwrap();
        assert!(whitespace[1].whitespace);

        let carry = aggregate(
            &values,
            0,
            180,
            &AggregationConfig::new(60, GapPolicy::CarryForward),
        )
        .unwrap();
        assert!(!carry[1].whitespace);
        assert!(carry[1].synthetic);
        assert_eq!(carry[1].close, 101);
        assert_eq!(carry[1].volume, 0);
    }

    #[test]
    fn rejects_checked_volume_overflow() {
        let values = [bar(0, 101, i64::MAX), bar(60, 102, 1)];
        assert_eq!(
            aggregate(
                &values,
                0,
                120,
                &AggregationConfig::new(120, GapPolicy::PreserveGaps),
            )
            .unwrap_err(),
            AggregationError::Overflow,
        );
    }
}
