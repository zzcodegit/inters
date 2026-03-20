use crate::crypto::{derive_aead_key, generate_x25519_keypair, AeadKey};
use crate::protocol::{encode, MsgType, TunnelMessage, PROTOCOL_VERSION};
use anyhow::Result;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey, StaticSecret};

/// Payload of a HandshakeInit message sent by the client.
/// When `cookie` is `Some`, client is replying to HandshakeChallenge (Stage 3.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeInitPayload {
    pub client_pubkey: [u8; 32],
    pub client_nonce: [u8; 32],
    /// Reserved for future transport / capability negotiation.
    pub transport_hint: String,
    /// Stage 3.1: set when resending init after receiving HandshakeChallenge.
    #[serde(default)]
    pub cookie: Option<[u8; 32]>,
}

/// Payload of a HandshakeChallenge message sent by the exit (Stage 3.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeChallengePayload {
    pub cookie: [u8; 32],
}

/// Payload of a HandshakeAck message sent by the exit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeAckPayload {
    pub exit_pubkey: [u8; 32],
    pub exit_nonce: [u8; 32],
    // Reserved for future negotiated parameters.
    pub negotiated_transport: String,
}

/// Client-side helper: build a HandshakeInit TunnelMessage and local keypair.
pub fn build_handshake_init(session_id: u32) -> (TunnelMessage, StaticSecret, PublicKey) {
    let kp = generate_x25519_keypair();
    let mut nonce = [0u8; 32];
    OsRng.fill_bytes(&mut nonce);

    let payload = HandshakeInitPayload {
        client_pubkey: kp.public.to_bytes(),
        client_nonce: nonce,
        transport_hint: "udp-native".to_string(),
        cookie: None,
    };
    let payload_bytes =
        bincode::serialize(&payload).expect("handshake init payload serialize");

    let msg = TunnelMessage::new(
        MsgType::HandshakeInit,
        session_id,
        0,
        0,
        payload_bytes,
    );

    (msg, kp.secret, kp.public)
}

/// Stage 3.1: build HandshakeInit with cookie (after receiving HandshakeChallenge).
pub fn build_handshake_init_with_cookie(
    session_id: u32,
    client_pubkey: [u8; 32],
    client_nonce: [u8; 32],
    cookie: [u8; 32],
) -> TunnelMessage {
    let payload = HandshakeInitPayload {
        client_pubkey,
        client_nonce,
        transport_hint: "udp-native".to_string(),
        cookie: Some(cookie),
    };
    let payload_bytes =
        bincode::serialize(&payload).expect("handshake init payload serialize");
    TunnelMessage::new(MsgType::HandshakeInit, session_id, 0, 0, payload_bytes)
}

/// Exit-side helper: process HandshakeInit and build HandshakeAck plus session AEAD key.
pub fn handle_handshake_init(
    init: &HandshakeInitPayload,
    session_id: u32,
) -> (TunnelMessage, StaticSecret, PublicKey, AeadKey) {
    let kp = generate_x25519_keypair();
    let mut nonce = [0u8; 32];
    OsRng.fill_bytes(&mut nonce);

    let client_pk = PublicKey::from(init.client_pubkey);
    let aead_key = derive_aead_key(&kp.secret, &client_pk);

    let ack = HandshakeAckPayload {
        exit_pubkey: kp.public.to_bytes(),
        exit_nonce: nonce,
        negotiated_transport: "udp-native".to_string(),
    };
    let payload_bytes =
        bincode::serialize(&ack).expect("handshake ack payload serialize");

    let msg = TunnelMessage::new(
        MsgType::HandshakeAck,
        session_id,
        0,
        0,
        payload_bytes,
    );

    (msg, kp.secret, kp.public, aead_key)
}

/// Client-side helper: consume HandshakeAck and derive the session AEAD key.
pub fn derive_session_key_from_ack(
    local_secret: &StaticSecret,
    ack: &HandshakeAckPayload,
) -> AeadKey {
    let exit_pk = PublicKey::from(ack.exit_pubkey);
    derive_aead_key(local_secret, &exit_pk)
}

/// Encode a handshake TunnelMessage as raw bytes without encryption.
pub fn encode_plaintext(msg: &TunnelMessage) -> Result<Vec<u8>> {
    if msg.header.version != PROTOCOL_VERSION {
        anyhow::bail!("unexpected protocol version in handshake message");
    }
    Ok(encode(msg)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionCrypto;
    use crate::protocol::Header;

    #[test]
    fn handshake_roundtrip_derives_matching_keys() {
        let session_id = 1;
        let (init_msg, client_secret, _client_pub) = build_handshake_init(session_id);

        // Exit decodes init payload and builds ack + its side of the key.
        let init_payload: HandshakeInitPayload =
            bincode::deserialize(&init_msg.payload).expect("decode init payload");
        let (ack_msg, _exit_secret, _exit_pub, exit_key) =
            handle_handshake_init(&init_payload, session_id);

        // Client processes ack and derives its session key.
        let ack_payload: HandshakeAckPayload =
            bincode::deserialize(&ack_msg.payload).expect("decode ack payload");
        let client_key = derive_session_key_from_ack(&client_secret, &ack_payload);

        // Use SessionCrypto on both sides to prove we can communicate.
        let c_crypto = SessionCrypto::new(client_key);
        let e_crypto = SessionCrypto::new(exit_key);

        let msg = TunnelMessage {
            header: Header {
                version: PROTOCOL_VERSION,
                msg_type: MsgType::Data,
                session_id,
                stream_id: 1,
                seq: 0,
                hop_index: 0,
            },
            payload: b"handshake-ok".to_vec(),
        };

        let ct = c_crypto.seal_message(msg.clone()).expect("seal");
        let dec = e_crypto.open_message(&ct).expect("open");
        assert_eq!(msg.payload, dec.payload);
    }
}


