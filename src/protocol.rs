use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MsgType {
    HandshakeInit = 0,
    HandshakeAck = 1,
    OpenStream = 2,
    Data = 3,
    CloseStream = 4,
    Error = 5,
    Ping = 6,
    Pong = 7,
    /// Stage 3.1: stateless anti-amplification; exit sends this instead of creating session.
    HandshakeChallenge = 8,
    /// Stage 7: measurement-only control-plane packet (ants).
    Ant = 9,
    /// Stage 9.1: discovery control-plane advertisement.
    DiscoveryAdvertise = 10,
    /// Stage 9.1: discovery query for known nodes.
    DiscoveryQuery = 11,
    /// Stage 9.1: discovery response with bounded advertisements.
    DiscoveryResponse = 12,
}

/// Stage 5: hop_index for variable-length circuits.
/// Client sets 0. Relays are blind (don't parse). Exit is final hop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Header {
    pub version: u8,
    pub msg_type: MsgType,
    pub session_id: u32,
    pub stream_id: u32,
    pub seq: u64,
    /// Stage 5: forward path hop index. Client=0. Default 0 for backward compat.
    #[serde(default)]
    pub hop_index: u8,
}

/// Stage 4: per-stream reliable framing header carried inside `TunnelMessage::payload` for DATA.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamFrame {
    pub stream_id: u32,
    pub frame_seq: u64,
    pub ack_seq: u64,
    pub payload: Vec<u8>,
}

/// Exit-side response-path quality summary delivered to the client when a
/// buffered HTTP stream closes. Used only for route scoring/diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResponseQualityFeedback {
    pub stream_id: u32,
    pub route_len: u8,
    pub resp_bytes: u64,
    pub frames_sent: u64,
    pub stream_duration_ms: u64,
    pub first_target_byte_ms: Option<u64>,
    pub first_overlay_send_ms: Option<u64>,
    pub ack_latency_ms_avg: Option<u64>,
    pub ack_latency_ms_p50: Option<u64>,
    pub ack_latency_ms_p95: Option<u64>,
    pub ack_latency_ms_max: Option<u64>,
    pub total_retransmits: u64,
    pub retransmit_rate_ppm: u32,
    pub window_wait_events: u64,
    pub window_wait_total_ms: u64,
    pub window_wait_max_ms: u64,
    pub http_code: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TunnelMessage {
    pub header: Header,
    pub payload: Vec<u8>,
}

pub const PROTOCOL_VERSION: u8 = 1;

impl TunnelMessage {
    pub fn new(
        msg_type: MsgType,
        session_id: u32,
        stream_id: u32,
        seq: u64,
        payload: Vec<u8>,
    ) -> Self {
        Self::with_hop(msg_type, session_id, stream_id, seq, 0, payload)
    }

    pub fn with_hop(
        msg_type: MsgType,
        session_id: u32,
        stream_id: u32,
        seq: u64,
        hop_index: u8,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            header: Header {
                version: PROTOCOL_VERSION,
                msg_type,
                session_id,
                stream_id,
                seq,
                hop_index,
            },
            payload,
        }
    }
}

pub fn encode(msg: &TunnelMessage) -> anyhow::Result<Vec<u8>> {
    Ok(bincode::serialize(msg)?)
}

pub fn decode(data: &[u8]) -> anyhow::Result<TunnelMessage> {
    Ok(bincode::deserialize(data)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let msg = TunnelMessage::new(MsgType::Data, 1, 2, 3, b"hello".to_vec());
        let enc = encode(&msg).unwrap();
        let dec = decode(&enc).unwrap();
        assert_eq!(msg, dec);
    }

    #[test]
    fn hop_index_in_header() {
        let msg = TunnelMessage::with_hop(MsgType::Data, 1, 2, 3, 2, b"x".to_vec());
        assert_eq!(msg.header.hop_index, 2);
        let enc = encode(&msg).unwrap();
        let dec = decode(&enc).unwrap();
        assert_eq!(dec.header.hop_index, 2);
    }

    #[test]
    fn stream_frame_roundtrip() {
        let frame = StreamFrame {
            stream_id: 42,
            frame_seq: 7,
            ack_seq: 5,
            payload: b"abc".to_vec(),
        };
        let enc = bincode::serialize(&frame).unwrap();
        let dec: StreamFrame = bincode::deserialize(&enc).unwrap();
        assert_eq!(frame, dec);
    }

    #[test]
    fn response_quality_feedback_roundtrip() {
        let feedback = ResponseQualityFeedback {
            stream_id: 17,
            route_len: 3,
            resp_bytes: 262_144,
            frames_sent: 256,
            stream_duration_ms: 1_234,
            first_target_byte_ms: Some(2),
            first_overlay_send_ms: Some(4),
            ack_latency_ms_avg: Some(120),
            ack_latency_ms_p50: Some(100),
            ack_latency_ms_p95: Some(240),
            ack_latency_ms_max: Some(300),
            total_retransmits: 3,
            retransmit_rate_ppm: 12_000,
            window_wait_events: 5,
            window_wait_total_ms: 180,
            window_wait_max_ms: 70,
            http_code: Some(200),
        };
        let enc = bincode::serialize(&feedback).unwrap();
        let dec: ResponseQualityFeedback = bincode::deserialize(&enc).unwrap();
        assert_eq!(feedback, dec);
    }
}
