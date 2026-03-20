use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    Open,
    HalfClosedLocal,
    HalfClosedRemote,
    Closed,
}

#[derive(Debug)]
pub struct Stream {
    pub id: u32,
    pub state: StreamState,
}

#[derive(Debug, Default)]
pub struct StreamTable {
    streams: HashMap<u32, Stream>,
}

impl StreamTable {
    pub fn open_stream(&mut self, id: u32) {
        self.streams.insert(
            id,
            Stream {
                id,
                state: StreamState::Open,
            },
        );
    }

    pub fn close_local(&mut self, id: u32) {
        if let Some(s) = self.streams.get_mut(&id) {
            s.state = match s.state {
                StreamState::Open => StreamState::HalfClosedLocal,
                StreamState::HalfClosedRemote => StreamState::Closed,
                other => other,
            };
        }
    }

    pub fn close_remote(&mut self, id: u32) {
        if let Some(s) = self.streams.get_mut(&id) {
            s.state = match s.state {
                StreamState::Open => StreamState::HalfClosedRemote,
                StreamState::HalfClosedLocal => StreamState::Closed,
                other => other,
            };
        }
    }

    pub fn remove(&mut self, id: u32) {
        self.streams.remove(&id);
    }

    pub fn get_state(&self, id: u32) -> Option<StreamState> {
        self.streams.get(&id).map(|s| s.state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_state_transitions() {
        let mut table = StreamTable::default();
        table.open_stream(1);
        assert_eq!(table.get_state(1), Some(StreamState::Open));
        table.close_local(1);
        assert_eq!(table.get_state(1), Some(StreamState::HalfClosedLocal));
        table.close_remote(1);
        assert_eq!(table.get_state(1), Some(StreamState::Closed));
        table.remove(1);
        assert_eq!(table.get_state(1), None);
    }
}

