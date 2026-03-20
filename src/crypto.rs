use anyhow::{anyhow, Result};
use chacha20poly1305::aead::{Aead, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

pub const AEAD_KEY_LEN: usize = 32;
// simple non-zero dev key; in real system this must be replaced
const DEV_PSK: [u8; AEAD_KEY_LEN] = [0x42; AEAD_KEY_LEN];

#[derive(Clone)]
pub struct AeadKey(pub Key);

#[derive(Clone)]
pub struct Keypair {
    pub secret: StaticSecret,
    pub public: PublicKey,
}

pub fn generate_x25519_keypair() -> Keypair {
    let secret = StaticSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&secret);
    Keypair { secret, public }
}

pub fn derive_aead_key(local_secret: &StaticSecret, peer_public: &PublicKey) -> AeadKey {
    let shared = local_secret.diffie_hellman(peer_public);
    let hk = Hkdf::<Sha256>::new(None, shared.as_bytes());
    let mut okm = [0u8; AEAD_KEY_LEN];
    hk.expand(b"vpnnode-session-key", &mut okm)
        .expect("hkdf expand");
    AeadKey(Key::from_slice(&okm).to_owned())
}

/// Development-only static AEAD key used for the initial MVP.
pub fn dev_default_aead_key() -> AeadKey {
    AeadKey(Key::from_slice(&DEV_PSK).to_owned())
}

pub fn seal(aead_key: &AeadKey, nonce_u64: u64, plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(&aead_key.0);
    let mut nonce_bytes = [0u8; 12];
    nonce_bytes[4..].copy_from_slice(&nonce_u64.to_be_bytes());
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| anyhow!("encrypt error: {e}"))?;
    Ok(ct)
}

pub fn open(aead_key: &AeadKey, nonce_u64: u64, ciphertext: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(&aead_key.0);
    let mut nonce_bytes = [0u8; 12];
    nonce_bytes[4..].copy_from_slice(&nonce_u64.to_be_bytes());
    let nonce = Nonce::from_slice(&nonce_bytes);
    let pt = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("decrypt error: {e}"))?;
    Ok(pt)
}

pub fn save_static_secret(path: &str, secret: &StaticSecret) -> Result<()> {
    use std::fs;
    use std::io::Write;

    let bytes = secret.to_bytes();
    let mut f = fs::File::create(path)?;
    f.write_all(&bytes)?;
    Ok(())
}

pub fn load_static_secret(path: &str) -> Result<StaticSecret> {
    use std::fs;
    use std::io::Read;
    let mut f = fs::File::open(path)?;
    let mut buf = [0u8; 32];
    f.read_exact(&mut buf)?;
    Ok(StaticSecret::from(buf))
}

pub fn save_public_key(path: &str, pk: &PublicKey) -> Result<()> {
    use std::fs;
    use std::io::Write;
    let mut f = fs::File::create(path)?;
    f.write_all(pk.as_bytes())?;
    Ok(())
}

pub fn load_public_key(path: &str) -> Result<PublicKey> {
    use std::io::Read;
    let mut f = std::fs::File::open(path)?;
    let mut buf = [0u8; 32];
    f.read_exact(&mut buf)?;
    Ok(PublicKey::from(buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seal_open_roundtrip() {
        let kp1 = generate_x25519_keypair();
        let kp2 = generate_x25519_keypair();
        let k1 = derive_aead_key(&kp1.secret, &kp2.public);
        let k2 = derive_aead_key(&kp2.secret, &kp1.public);

        let msg = b"hello vpnnode";
        let ct = seal(&k1, 42, msg).unwrap();
        let pt = open(&k2, 42, &ct).unwrap();
        assert_eq!(pt, msg);
    }
}

