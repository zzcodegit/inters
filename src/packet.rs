use anyhow::{bail, Result};

/// Very small IPv4 header representation for test / TUN purposes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ipv4Header {
    pub src: [u8; 4],
    pub dst: [u8; 4],
    pub protocol: u8,
    pub header_len: u8,
}

impl Ipv4Header {
    pub fn parse(buf: &[u8]) -> Result<(Self, usize)> {
        if buf.len() < 20 {
            bail!("buffer too small for IPv4 header");
        }
        let version_ihl = buf[0];
        let version = version_ihl >> 4;
        if version != 4 {
            bail!("not IPv4 packet");
        }
        let ihl = version_ihl & 0x0f;
        let header_bytes = (ihl as usize) * 4;
        if buf.len() < header_bytes {
            bail!("truncated IPv4 header");
        }
        let protocol = buf[9];
        let src = [buf[12], buf[13], buf[14], buf[15]];
        let dst = [buf[16], buf[17], buf[18], buf[19]];
        Ok((
            Ipv4Header {
                src,
                dst,
                protocol,
                header_len: ihl,
            },
            header_bytes,
        ))
    }
}

/// Flow key for mapping a 4‑tuple (src, dst, src_port, dst_port) to a tunnel stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FlowKey {
    pub src: [u8; 4],
    pub dst: [u8; 4],
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,
}

impl FlowKey {
    pub fn new(
        src: [u8; 4],
        dst: [u8; 4],
        src_port: u16,
        dst_port: u16,
        protocol: u8,
    ) -> Self {
        Self {
            src,
            dst,
            src_port,
            dst_port,
            protocol,
        }
    }
}

/// Minimal TCP header parser used only to derive ports for flow keys.
pub fn parse_tcp_ports(buf: &[u8]) -> Result<(u16, u16)> {
    if buf.len() < 4 {
        bail!("buffer too small for TCP header");
    }
    let src_port = u16::from_be_bytes([buf[0], buf[1]]);
    let dst_port = u16::from_be_bytes([buf[2], buf[3]]);
    Ok((src_port, dst_port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_parse_roundtrip_basic() {
        // Minimal IPv4 header for testing: version 4, IHL 5, protocol TCP(6)
        let mut buf = [0u8; 20];
        buf[0] = (4 << 4) | 5;
        buf[9] = 6;
        buf[12..16].copy_from_slice(&[10, 0, 0, 1]);
        buf[16..20].copy_from_slice(&[10, 0, 0, 2]);

        let (hdr, consumed) = Ipv4Header::parse(&buf).unwrap();
        assert_eq!(consumed, 20);
        assert_eq!(hdr.src, [10, 0, 0, 1]);
        assert_eq!(hdr.dst, [10, 0, 0, 2]);
        assert_eq!(hdr.protocol, 6);
        assert_eq!(hdr.header_len, 5);
    }

    #[test]
    fn tcp_ports_parse_basic() {
        let buf = [0x1f, 0x90, 0x00, 0x50]; // 8080 -> 80
        let (src, dst) = parse_tcp_ports(&buf).unwrap();
        assert_eq!(src, 8080);
        assert_eq!(dst, 80);
    }
}

