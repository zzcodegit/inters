use std::time::{Duration, Instant};

use vpnnode::flow::FlowTable;
use vpnnode::packet::{parse_tcp_ports, FlowKey, Ipv4Header};

/// Cross-platform logic tests for the FlowTable and basic packet helpers
/// used by TUN mode. These do not require a real TUN device.
#[test]
fn flow_table_roundtrip_flow_and_stream() {
    let mut table = FlowTable::default();
    let mut next = 1u32;
    let now = Instant::now();

    let key = FlowKey::new([10, 0, 0, 1], [10, 0, 0, 2], 12345, 80, 6);
    let sid = table.get_or_create(key, &mut next, now);

    let looked_up = table
        .get_flow_for_stream(sid)
        .expect("flow must be present for stream");
    assert_eq!(looked_up, key);

    table.remove_by_stream(sid);
    assert!(table.get_flow_for_stream(sid).is_none());
}

#[test]
fn flow_table_idle_timeout_behavior() {
    let mut table = FlowTable::default();
    let mut next = 1u32;
    let now = Instant::now();

    let key_active = FlowKey::new([10, 0, 0, 1], [10, 0, 0, 2], 1111, 80, 6);
    let key_idle = FlowKey::new([10, 0, 0, 3], [10, 0, 0, 4], 2222, 80, 6);

    let sid_active = table.get_or_create(key_active, &mut next, now);
    let sid_idle = table.get_or_create(key_idle, &mut next, now);
    assert_ne!(sid_active, sid_idle);

    // Simulate time passing and mark one flow as active.
    let later = now + Duration::from_secs(20);
    table.touch_by_stream(sid_active, later);

    let removed = table.prune_idle(Duration::from_secs(10), later);
    assert_eq!(removed, 1, "exactly one idle flow must be pruned");
    assert!(table.get_flow_for_stream(sid_idle).is_none());
    assert_eq!(
        table.get_flow_for_stream(sid_active),
        Some(key_active),
        "active flow must remain after pruning"
    );
}

#[test]
fn ipv4_and_tcp_helpers_basic() {
    // Minimal IPv4 header for testing: version 4, IHL 5, protocol TCP(6)
    let mut buf = [0u8; 24];
    buf[0] = (4 << 4) | 5;
    buf[9] = 6;
    buf[12..16].copy_from_slice(&[10, 0, 0, 1]);
    buf[16..20].copy_from_slice(&[10, 0, 0, 2]);
    // Append minimal TCP header (ports only)
    buf[20] = 0x1f;
    buf[21] = 0x90; // 8080
    buf[22] = 0x00;
    buf[23] = 0x50; // 80

    let (ip, consumed) = Ipv4Header::parse(&buf).expect("parse ipv4 header");
    assert_eq!(consumed, 20);
    assert_eq!(ip.src, [10, 0, 0, 1]);
    assert_eq!(ip.dst, [10, 0, 0, 2]);
    assert_eq!(ip.protocol, 6);

    let (src_port, dst_port) = parse_tcp_ports(&buf[consumed..]).expect("parse tcp ports");
    assert_eq!(src_port, 8080);
    assert_eq!(dst_port, 80);
}

