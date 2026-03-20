use crate::packet::FlowKey;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
struct FlowEntry {
    stream_id: u32,
    last_seen: Instant,
}

/// Bidirectional flow table mapping FlowKey <-> stream_id with basic
/// idle-timeout based cleanup support.
#[derive(Debug, Default)]
pub struct FlowTable {
    flow_to_stream: HashMap<FlowKey, FlowEntry>,
    stream_to_flow: HashMap<u32, FlowKey>,
}

impl FlowTable {
    /// Lookup an existing stream id for the given flow, or allocate a new
    /// stream id from `next_stream_id` and install the mapping. The flow's
    /// last_seen timestamp is updated to `now`.
    pub fn get_or_create(
        &mut self,
        key: FlowKey,
        next_stream_id: &mut u32,
        now: Instant,
    ) -> u32 {
        if let Some(entry) = self.flow_to_stream.get_mut(&key) {
            entry.last_seen = now;
            entry.stream_id
        } else {
            let id = *next_stream_id;
            *next_stream_id += 1;
            self.flow_to_stream.insert(
                key,
                FlowEntry {
                    stream_id: id,
                    last_seen: now,
                },
            );
            self.stream_to_flow.insert(id, key);
            id
        }
    }

    /// Lookup the flow associated with a given stream id.
    pub fn get_flow_for_stream(&self, stream_id: u32) -> Option<FlowKey> {
        self.stream_to_flow.get(&stream_id).cloned()
    }

    /// Mark an existing flow (by stream id) as active "now".
    pub fn touch_by_stream(&mut self, stream_id: u32, now: Instant) {
        if let Some(flow) = self.stream_to_flow.get(&stream_id).cloned() {
            if let Some(entry) = self.flow_to_stream.get_mut(&flow) {
                entry.last_seen = now;
            }
        }
    }

    /// Remove a mapping by flow key.
    pub fn remove_by_flow(&mut self, key: &FlowKey) {
        if let Some(entry) = self.flow_to_stream.remove(key) {
            self.stream_to_flow.remove(&entry.stream_id);
        }
    }

    /// Remove a mapping by stream id.
    pub fn remove_by_stream(&mut self, stream_id: u32) {
        if let Some(flow) = self.stream_to_flow.remove(&stream_id) {
            self.flow_to_stream.remove(&flow);
        }
    }

    /// Remove flows that have been idle for at least `max_idle` according to
    /// `now`. Returns the number of removed entries.
    pub fn prune_idle(&mut self, max_idle: Duration, now: Instant) -> usize {
        if max_idle.is_zero() {
            return 0;
        }
        let mut removed = Vec::new();
        for (flow, entry) in self.flow_to_stream.iter() {
            if now.saturating_duration_since(entry.last_seen) >= max_idle {
                removed.push((*flow, entry.stream_id));
            }
        }
        let count = removed.len();
        for (flow, stream_id) in removed {
            self.flow_to_stream.remove(&flow);
            self.stream_to_flow.remove(&stream_id);
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_table_basic_bidirectional_mapping() {
        let mut table = FlowTable::default();
        let mut next = 1u32;
        let key = FlowKey::new([10, 0, 0, 1], [10, 0, 0, 2], 12345, 80, 6);
        let now = Instant::now();

        let s1 = table.get_or_create(key, &mut next, now);
        let s2 = table.get_or_create(key, &mut next, now);
        assert_eq!(s1, s2, "same flow must reuse stream id");
        assert_eq!(next, s1 + 1);

        let looked_up = table.get_flow_for_stream(s1).expect("flow for stream");
        assert_eq!(looked_up, key);

        table.remove_by_flow(&key);
        assert!(table.get_flow_for_stream(s1).is_none());

        // Re-create should yield a new stream id and new mapping.
        let later = now + Duration::from_secs(1);
        let s3 = table.get_or_create(key, &mut next, later);
        assert_ne!(s1, s3);
        assert_eq!(
            table.get_flow_for_stream(s3),
            Some(key),
            "new stream id should be mapped back to flow"
        );

        table.remove_by_stream(s3);
        assert!(table.get_flow_for_stream(s3).is_none());
    }

    #[test]
    fn flow_table_idle_pruning_removes_stale_flows() {
        let mut table = FlowTable::default();
        let mut next = 1u32;
        let now = Instant::now();

        let key1 = FlowKey::new([10, 0, 0, 1], [10, 0, 0, 2], 1111, 80, 6);
        let key2 = FlowKey::new([10, 0, 0, 3], [10, 0, 0, 4], 2222, 80, 6);

        let s1 = table.get_or_create(key1, &mut next, now);
        let s2 = table.get_or_create(key2, &mut next, now);
        assert_ne!(s1, s2);

        // Advance "time" for pruning purposes.
        let later = now + Duration::from_secs(30);

        // Touch second flow so it stays active.
        table.touch_by_stream(s2, later);

        let removed = table.prune_idle(Duration::from_secs(10), later);
        assert_eq!(removed, 1);
        assert!(table.get_flow_for_stream(s1).is_none());
        assert_eq!(table.get_flow_for_stream(s2), Some(key2));
    }
}

