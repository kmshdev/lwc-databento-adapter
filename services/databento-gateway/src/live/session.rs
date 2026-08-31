use std::collections::{HashMap, VecDeque};

use tokio::sync::mpsc;

use crate::protocol::{Resolution, SourceSchema, SymbolType};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResolvedStreamKeyLike {
    pub dataset: String,
    pub requested_symbol: String,
    pub stype_in: SymbolType,
    pub resolution: Resolution,
    pub gap_policy: crate::protocol::GapPolicy,
    pub resolved_symbol: String,
    pub instrument_id: i64,
    pub source_schema: SourceSchema,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandoffResolution {
    Continue,
    RestartWithChangedInstrument,
}

/// A deterministic bounded downstream policy.  Only same-bucket bar mutations
/// may replace each other; lifecycle and mapping events must never be dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownstreamEventKind {
    Bar { bucket: i64 },
    Lifecycle,
    Mapping,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueuePush {
    Enqueued,
    Coalesced,
    SlowConsumer,
}

/// Process-scoped ownership for the Live connection of a dataset.  The
/// registry deliberately separates a *dataset actor* (one physical client)
/// from canonical requests within that actor.  A new canonical request is an
/// instruction for the actor to add/resubscribe that request; it is never a
/// reason for transport code to construct another client for the same dataset.
///
/// `T` is the normalized event placed on bounded canonical queues.  Keeping
/// this generic makes the lifecycle and ref-counting rules deterministic in
/// offline tests and prevents the registry from learning about DBN records.
#[derive(Debug)]
pub struct DatasetLiveRegistry<T> {
    max_datasets: usize,
    queue_capacity: usize,
    datasets: HashMap<String, DatasetLiveActor<T>>,
}

#[derive(Debug)]
struct DatasetLiveActor<T> {
    canonical: HashMap<ResolvedStreamKeyLike, CanonicalSubscription<T>>,
}

#[derive(Debug)]
struct CanonicalSubscription<T> {
    downstream: HashMap<String, mpsc::Sender<T>>,
    // The actor has only one physical client, so a newly attached downstream
    // cannot request an independent upstream replay. Keep a bounded canonical
    // tail and use a transport-supplied replay marker to establish its resume
    // boundary without violating that ownership rule.
    replay_tail: VecDeque<T>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryAcquireError {
    DatasetLimit,
}

pub struct RegistryLease<T> {
    pub receiver: mpsc::Receiver<T>,
    /// True only for the first downstream for this exact canonical request.
    /// The dataset actor uses this to add the canonical request upstream.
    pub canonical_added: bool,
    /// True only for the first request in a dataset.  The composition root
    /// uses this to construct the sole physical `LiveClient` for that dataset.
    pub dataset_added: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegistryRelease {
    pub canonical_released: bool,
    pub dataset_released: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryPublish {
    pub slow_consumers: Vec<String>,
}

impl<T> DatasetLiveRegistry<T> {
    pub fn new(max_datasets: usize, queue_capacity: usize) -> Self {
        assert!(max_datasets > 0, "dataset limit must be positive");
        assert!(
            queue_capacity > 0,
            "canonical queue capacity must be positive"
        );
        Self {
            max_datasets,
            queue_capacity,
            datasets: HashMap::new(),
        }
    }

    pub fn active_dataset_count(&self) -> usize {
        self.datasets.len()
    }

    pub fn active_canonical_count(&self, dataset: &str) -> usize {
        self.datasets
            .get(dataset)
            .map_or(0, |actor| actor.canonical.len())
    }

    pub fn downstream_count(&self, dataset: &str, key: &ResolvedStreamKeyLike) -> usize {
        self.datasets
            .get(dataset)
            .and_then(|actor| actor.canonical.get(key))
            .map_or(0, |canonical| canonical.downstream.len())
    }

    pub fn acquire(
        &mut self,
        dataset: &str,
        key: ResolvedStreamKeyLike,
        downstream_id: String,
    ) -> Result<RegistryLease<T>, RegistryAcquireError> {
        let dataset_added = !self.datasets.contains_key(dataset);
        if dataset_added && self.datasets.len() == self.max_datasets {
            return Err(RegistryAcquireError::DatasetLimit);
        }
        let actor = self
            .datasets
            .entry(dataset.to_string())
            .or_insert_with(|| DatasetLiveActor {
                canonical: HashMap::new(),
            });
        let canonical_added = !actor.canonical.contains_key(&key);
        let canonical = actor
            .canonical
            .entry(key)
            .or_insert_with(|| CanonicalSubscription {
                downstream: HashMap::new(),
                replay_tail: VecDeque::new(),
            });
        let (sender, receiver) = mpsc::channel(self.queue_capacity);
        canonical.downstream.insert(downstream_id, sender);
        Ok(RegistryLease {
            receiver,
            canonical_added,
            dataset_added,
        })
    }

    pub fn release(
        &mut self,
        dataset: &str,
        key: &ResolvedStreamKeyLike,
        downstream_id: &str,
    ) -> RegistryRelease {
        let mut canonical_released = false;
        let mut dataset_released = false;
        if let Some(actor) = self.datasets.get_mut(dataset) {
            if let Some(canonical) = actor.canonical.get_mut(key) {
                canonical.downstream.remove(downstream_id);
                if canonical.downstream.is_empty() {
                    actor.canonical.remove(key);
                    canonical_released = true;
                }
            }
            if actor.canonical.is_empty() {
                self.datasets.remove(dataset);
                dataset_released = true;
            }
        }
        RegistryRelease {
            canonical_released,
            dataset_released,
        }
    }
}

impl<T: Clone> DatasetLiveRegistry<T> {
    /// Like [`Self::acquire`], but when the canonical stream already exists it
    /// seeds the new downstream with the bounded canonical tail followed by a
    /// replay-complete marker. This makes reconnect handoff deterministic while
    /// retaining one physical client per dataset.
    pub fn acquire_with_replay_boundary(
        &mut self,
        dataset: &str,
        key: ResolvedStreamKeyLike,
        downstream_id: String,
        replay_boundary: T,
    ) -> Result<RegistryLease<T>, RegistryAcquireError> {
        let dataset_added = !self.datasets.contains_key(dataset);
        if dataset_added && self.datasets.len() == self.max_datasets {
            return Err(RegistryAcquireError::DatasetLimit);
        }
        let actor = self
            .datasets
            .entry(dataset.to_string())
            .or_insert_with(|| DatasetLiveActor {
                canonical: HashMap::new(),
            });
        let canonical_added = !actor.canonical.contains_key(&key);
        let canonical = actor
            .canonical
            .entry(key)
            .or_insert_with(|| CanonicalSubscription {
                downstream: HashMap::new(),
                replay_tail: VecDeque::new(),
            });
        let (sender, receiver) = mpsc::channel(self.queue_capacity);
        if !canonical_added {
            for event in canonical.replay_tail.iter().cloned() {
                sender
                    .try_send(event)
                    .expect("bounded replay tail fits its downstream queue");
            }
            sender
                .try_send(replay_boundary)
                .expect("replay marker fits after bounded tail");
        }
        canonical.downstream.insert(downstream_id, sender);
        Ok(RegistryLease {
            receiver,
            canonical_added,
            dataset_added,
        })
    }

    /// Fan-out never waits on a browser. A full bounded queue identifies that
    /// downstream as slow; the WebSocket layer emits its typed error and
    /// removes it without affecting other clients or the dataset actor.
    pub fn publish(
        &mut self,
        dataset: &str,
        key: &ResolvedStreamKeyLike,
        event: T,
    ) -> RegistryPublish {
        self.publish_with_replay_retention(dataset, key, event, true)
    }

    /// Publishes an event while allowing lifecycle markers to bypass the
    /// replay tail. A resumed downstream receives exactly one fresh replay
    /// boundary, never a stale boundary retained from the original session.
    pub fn publish_with_replay_retention(
        &mut self,
        dataset: &str,
        key: &ResolvedStreamKeyLike,
        event: T,
        retain_for_replay: bool,
    ) -> RegistryPublish {
        let Some(canonical) = self
            .datasets
            .get_mut(dataset)
            .and_then(|actor| actor.canonical.get_mut(key))
        else {
            return RegistryPublish {
                slow_consumers: Vec::new(),
            };
        };

        if retain_for_replay {
            canonical.replay_tail.push_back(event.clone());
            while canonical.replay_tail.len() >= self.queue_capacity {
                canonical.replay_tail.pop_front();
            }
        }

        let mut slow_consumers = Vec::new();
        for (id, sender) in &canonical.downstream {
            if sender.try_send(event.clone()).is_err() {
                slow_consumers.push(id.clone());
            }
        }
        RegistryPublish { slow_consumers }
    }
}

#[derive(Debug)]
pub struct DownstreamQueue<T> {
    capacity: usize,
    entries: Vec<(DownstreamEventKind, T)>,
}

impl<T> DownstreamQueue<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "downstream capacity must be positive");
        Self {
            capacity,
            entries: Vec::new(),
        }
    }

    pub fn push(&mut self, kind: DownstreamEventKind, value: T) -> QueuePush {
        if let DownstreamEventKind::Bar { bucket } = kind {
            if let Some((_, existing)) = self.entries.iter_mut().rev().find(|(existing, _)| {
                matches!(existing, DownstreamEventKind::Bar { bucket: previous } if *previous == bucket)
            }) {
                *existing = value;
                return QueuePush::Coalesced;
            }
        }
        if self.entries.len() == self.capacity {
            return QueuePush::SlowConsumer;
        }
        self.entries.push((kind, value));
        QueuePush::Enqueued
    }

    pub fn drain(&mut self) -> Vec<T> {
        self.entries.drain(..).map(|(_, value)| value).collect()
    }
}

#[derive(Debug)]
struct DatasetSession {
    by_key: HashMap<ResolvedStreamKeyLike, usize>,
}

impl DatasetSession {
    fn new() -> Self {
        Self {
            by_key: HashMap::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct DatasetSessionManager {
    datasets: HashMap<String, DatasetSession>,
}

impl DatasetSessionManager {
    pub fn new() -> Self {
        Self {
            datasets: HashMap::new(),
        }
    }

    pub fn active_dataset_count(&self) -> usize {
        self.datasets.len()
    }

    pub fn active_subscription_count(&self) -> usize {
        self.datasets
            .values()
            .map(|session| session.by_key.values().copied().sum::<usize>())
            .sum()
    }

    pub fn acquire(&mut self, dataset: &str, key: ResolvedStreamKeyLike) -> usize {
        let session = self
            .datasets
            .entry(dataset.to_string())
            .or_insert_with(DatasetSession::new);
        let count = session.by_key.entry(key).or_insert(0);
        *count += 1;
        *count
    }

    pub fn release(&mut self, dataset: &str, key: &ResolvedStreamKeyLike) -> usize {
        let removed = if let Some(session) = self.datasets.get_mut(dataset) {
            if let Some(count) = session.by_key.get_mut(key) {
                if *count > 0 {
                    *count -= 1;
                }
                let remaining = *count;
                if remaining == 0 {
                    session.by_key.remove(key);
                }
                remaining
            } else {
                0
            }
        } else {
            0
        };

        if let Some(session) = self.datasets.get(dataset) {
            if session.by_key.is_empty() {
                self.datasets.remove(dataset);
            }
        }
        removed
    }

    pub fn is_empty(&self, dataset: &str) -> bool {
        self.datasets
            .get(dataset)
            .is_none_or(|session| session.by_key.is_empty())
    }

    pub fn check_resolved_instrument_change(
        &self,
        dataset: &str,
        requested_symbol: &str,
        stype: &SymbolType,
        new_resolved: (&str, i64),
    ) -> HandoffResolution {
        let Some(session) = self.datasets.get(dataset) else {
            return HandoffResolution::Continue;
        };

        let matches_request = session
            .by_key
            .keys()
            .filter(|existing| {
                existing.requested_symbol == requested_symbol && &existing.stype_in == stype
            })
            .collect::<Vec<_>>();

        if matches_request.iter().all(|existing| {
            existing.resolved_symbol == new_resolved.0 && existing.instrument_id == new_resolved.1
        }) {
            HandoffResolution::Continue
        } else {
            HandoffResolution::RestartWithChangedInstrument
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{GapPolicy, Resolution, SourceSchema, SymbolType};

    fn synthetic_key(symbol: &str, resolved: &str, instrument_id: i64) -> ResolvedStreamKeyLike {
        ResolvedStreamKeyLike {
            dataset: "GLBX.MDP3".to_string(),
            requested_symbol: symbol.to_string(),
            stype_in: SymbolType::RawSymbol,
            resolution: Resolution::OneMinute,
            gap_policy: GapPolicy::PreserveGaps,
            resolved_symbol: resolved.to_string(),
            instrument_id,
            source_schema: SourceSchema::Ohlcv1m,
        }
    }

    #[test]
    fn refcounting_shares_dataset_session() {
        let mut manager = DatasetSessionManager::new();
        let key = synthetic_key("ESZ4", "ESZ4", 123);
        assert_eq!(manager.acquire("GLBX.MDP3", key.clone()), 1);
        assert_eq!(manager.acquire("GLBX.MDP3", key.clone()), 2);
        assert_eq!(manager.active_dataset_count(), 1);
        assert_eq!(manager.active_subscription_count(), 2);
        assert_eq!(manager.release("GLBX.MDP3", &key), 1);
        assert_eq!(manager.release("GLBX.MDP3", &key), 0);
        assert!(manager.is_empty("GLBX.MDP3"));
    }

    #[test]
    fn refcounting_is_dataset_scoped() {
        let mut manager = DatasetSessionManager::new();
        let first = synthetic_key("ESZ4", "ESZ4", 123);
        let second = synthetic_key("ESZ4", "ESZ4", 999);
        manager.acquire("GLBX.MDP3", first);
        manager.acquire("XNAS.ITCH", second);
        assert_eq!(manager.active_dataset_count(), 2);
        assert_eq!(manager.active_subscription_count(), 2);
        assert!(!manager.is_empty("GLBX.MDP3"));
        assert!(!manager.is_empty("XNAS.ITCH"));
    }

    #[test]
    fn resolved_instrument_changed_when_reconnect_mapping_moves() {
        let mut manager = DatasetSessionManager::new();
        let existing = synthetic_key("ES.FUT", "ESZ4", 123);
        manager.acquire("GLBX.MDP3", existing);

        assert_eq!(
            manager.check_resolved_instrument_change(
                "GLBX.MDP3",
                "ES.FUT",
                &SymbolType::RawSymbol,
                ("ESZ5", 124),
            ),
            HandoffResolution::RestartWithChangedInstrument,
        );
        assert_eq!(
            manager.check_resolved_instrument_change(
                "GLBX.MDP3",
                "ES.FUT",
                &SymbolType::RawSymbol,
                ("ESZ4", 123),
            ),
            HandoffResolution::Continue,
        );
    }

    #[tokio::test]
    async fn registry_shares_a_canonical_request_between_two_clients() {
        let mut registry = DatasetLiveRegistry::new(2, 2);
        let key = synthetic_key("ESZ4", "ESZ4", 123);
        let mut first = registry
            .acquire("GLBX.MDP3", key.clone(), "client-a/sub-a".to_string())
            .unwrap();
        let mut second = registry
            .acquire("GLBX.MDP3", key.clone(), "client-b/sub-b".to_string())
            .unwrap();
        assert!(first.dataset_added);
        assert!(first.canonical_added);
        assert!(!second.dataset_added);
        assert!(!second.canonical_added);
        assert_eq!(registry.active_dataset_count(), 1);
        assert_eq!(registry.active_canonical_count("GLBX.MDP3"), 1);
        assert_eq!(registry.downstream_count("GLBX.MDP3", &key), 2);

        registry.publish("GLBX.MDP3", &key, 7_u8);
        assert_eq!(first.receiver.recv().await, Some(7));
        assert_eq!(second.receiver.recv().await, Some(7));

        let release = registry.release("GLBX.MDP3", &key, "client-a/sub-a");
        assert!(!release.canonical_released);
        assert!(!release.dataset_released);
        let release = registry.release("GLBX.MDP3", &key, "client-b/sub-b");
        assert!(release.canonical_released);
        assert!(release.dataset_released);
    }

    #[tokio::test]
    async fn registry_replays_bounded_tail_then_boundary_for_shared_canonical_resume() {
        let mut registry = DatasetLiveRegistry::new(1, 4);
        let key = synthetic_key("ESZ4", "ESZ4", 123);
        let mut original = registry
            .acquire("GLBX.MDP3", key.clone(), "client-a/sub-a".to_string())
            .unwrap();
        for event in [10_u8, 11, 12] {
            registry.publish("GLBX.MDP3", &key, event);
            assert_eq!(original.receiver.recv().await, Some(event));
        }

        let mut resumed = registry
            .acquire_with_replay_boundary("GLBX.MDP3", key, "client-b/sub-b".to_string(), 255_u8)
            .unwrap();
        assert!(!resumed.canonical_added);
        let restored = vec![
            resumed.receiver.recv().await.unwrap(),
            resumed.receiver.recv().await.unwrap(),
            resumed.receiver.recv().await.unwrap(),
            resumed.receiver.recv().await.unwrap(),
        ];
        assert_eq!(restored, vec![10, 11, 12, 255]);
    }

    #[tokio::test]
    async fn registry_does_not_retain_stale_replay_boundaries() {
        let mut registry = DatasetLiveRegistry::new(1, 4);
        let key = synthetic_key("ESZ4", "ESZ4", 123);
        let mut original = registry
            .acquire("GLBX.MDP3", key.clone(), "client-a/sub-a".to_string())
            .unwrap();
        registry.publish_with_replay_retention("GLBX.MDP3", &key, 255_u8, false);
        assert_eq!(original.receiver.recv().await, Some(255));
        for event in [10_u8, 11] {
            registry.publish_with_replay_retention("GLBX.MDP3", &key, event, true);
            assert_eq!(original.receiver.recv().await, Some(event));
        }

        let mut resumed = registry
            .acquire_with_replay_boundary("GLBX.MDP3", key, "client-b/sub-b".to_string(), 255_u8)
            .unwrap();
        assert_eq!(resumed.receiver.recv().await, Some(10));
        assert_eq!(resumed.receiver.recv().await, Some(11));
        assert_eq!(resumed.receiver.recv().await, Some(255));
    }

    #[tokio::test]
    async fn registry_uses_one_dataset_actor_for_different_canonical_requests() {
        let mut registry = DatasetLiveRegistry::new(1, 1);
        let first_key = synthetic_key("ESZ4", "ESZ4", 123);
        let second_key = synthetic_key("NQZ4", "NQZ4", 456);
        let mut first = registry
            .acquire("GLBX.MDP3", first_key.clone(), "client-a/sub-a".to_string())
            .unwrap();
        let mut second = registry
            .acquire(
                "GLBX.MDP3",
                second_key.clone(),
                "client-b/sub-b".to_string(),
            )
            .unwrap();
        assert!(first.dataset_added);
        assert!(!second.dataset_added);
        assert!(first.canonical_added);
        assert!(second.canonical_added);
        assert_eq!(registry.active_dataset_count(), 1);
        assert_eq!(registry.active_canonical_count("GLBX.MDP3"), 2);

        registry.publish("GLBX.MDP3", &first_key, 1_u8);
        registry.publish("GLBX.MDP3", &second_key, 2_u8);
        assert_eq!(first.receiver.recv().await, Some(1));
        assert_eq!(second.receiver.recv().await, Some(2));
    }

    #[tokio::test]
    async fn registry_enforces_dataset_and_canonical_queue_bounds() {
        let mut registry = DatasetLiveRegistry::new(1, 1);
        let key = synthetic_key("ESZ4", "ESZ4", 123);
        let mut lease = registry
            .acquire("GLBX.MDP3", key.clone(), "slow".to_string())
            .unwrap();
        registry.publish("GLBX.MDP3", &key, 1_u8);
        let result = registry.publish("GLBX.MDP3", &key, 2_u8);
        assert_eq!(result.slow_consumers, vec!["slow"]);
        assert_eq!(lease.receiver.recv().await, Some(1));

        let other = synthetic_key("AAPL", "AAPL", 1);
        assert!(matches!(
            registry.acquire("XNAS.ITCH", other, "other".to_string()),
            Err(RegistryAcquireError::DatasetLimit)
        ));
    }

    #[test]
    fn queue_coalesces_only_same_bucket_bars() {
        let mut queue = DownstreamQueue::new(2);
        assert_eq!(
            queue.push(DownstreamEventKind::Bar { bucket: 60 }, 1),
            QueuePush::Enqueued
        );
        assert_eq!(
            queue.push(DownstreamEventKind::Bar { bucket: 60 }, 2),
            QueuePush::Coalesced
        );
        assert_eq!(
            queue.push(DownstreamEventKind::Lifecycle, 3),
            QueuePush::Enqueued
        );
        assert_eq!(queue.drain(), vec![2, 3]);
    }

    #[test]
    fn queue_rejects_full_lifecycle_without_dropping_it() {
        let mut queue = DownstreamQueue::new(1);
        assert_eq!(
            queue.push(DownstreamEventKind::Lifecycle, 1),
            QueuePush::Enqueued
        );
        assert_eq!(
            queue.push(DownstreamEventKind::Mapping, 2),
            QueuePush::SlowConsumer
        );
        assert_eq!(queue.drain(), vec![1]);
    }
}
