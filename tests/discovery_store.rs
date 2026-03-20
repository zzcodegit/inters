use vpnnode::addr::{NodeAddr, Protocol};
use vpnnode::discovery::{build_discovery_response_payload, make_self_advertisement, new_store};
use vpnnode::discovery::types::NodeId;
use vpnnode::node_config::NodeRole;
use std::net::SocketAddr;
use vpnnode::discovery::DiscoveryQueryPayload;

fn mk_addr(port: u16) -> NodeAddr {
    let sa: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    NodeAddr::from(sa)
}

fn mk_id(byte: u8) -> NodeId {
    [byte; 32]
}

#[test]
fn advertisement_freshness_and_expiry() {
    let id = mk_id(1);
    let adv = make_self_advertisement(
        id,
        NodeRole::Relay,
        mk_addr(30000),
        "vtest".to_string(),
        1000,
    );
    let now = adv.advertised_at_ms;
    assert!(adv.is_fresh(now));
    assert!(adv.is_fresh(now + 500));
    assert!(!adv.is_fresh(now + 2000));
}

#[test]
fn discovery_store_insert_and_purge() {
    let mut store = new_store(4);
    let id = mk_id(2);
    let adv = make_self_advertisement(
        id,
        NodeRole::Exit,
        mk_addr(30001),
        "vtest".to_string(),
        10,
    );
    let ts = adv.advertised_at_ms;
    store.insert(adv);
    assert_eq!(store.all_known().len(), 1);
    store.purge_expired(ts + 20);
    assert_eq!(store.all_known().len(), 0);
}

#[test]
fn discovery_store_bounded_and_filtered() {
    let mut store = new_store(2);

    let mut last_adv_ts = 0u64;

    for (i, role) in [NodeRole::Relay, NodeRole::Exit, NodeRole::Relay].into_iter().enumerate() {
        let adv = make_self_advertisement(
            mk_id(10 + i as u8),
            role,
            mk_addr(30000 + i as u16),
            "vtest".to_string(),
            10_000,
        );
        last_adv_ts = adv.advertised_at_ms;
        store.insert(adv);
    }

    let now = last_adv_ts + 100;
    let relays = store.fresh_candidates(now, Some(Protocol::Udp), Some(NodeRole::Relay), 8);
    assert!(!relays.is_empty());
    assert!(relays.iter().all(|a| a.role == NodeRole::Relay));

    let limited = store.fresh_candidates(now, None, None, 1);
    assert_eq!(limited.len(), 1);
}

#[test]
fn discovery_query_response_is_bounded() {
    let mut store = new_store(128);
    let base_ts = vpnnode::ant::now_ms();
    let ttl_ms = 10_000u64;

    for i in 0..50u8 {
        let role = if i % 2 == 0 { NodeRole::Relay } else { NodeRole::Exit };
        let adv = make_self_advertisement(
            mk_id(20 + i),
            role,
            mk_addr(31000 + i as u16),
            "vtest".to_string(),
            ttl_ms,
        );
        // Force deterministic ordering.
        let mut adv = adv;
        adv.advertised_at_ms = base_ts + i as u64;
        store.insert(adv);
    }

    let now = base_ts + 500;
    let query = DiscoveryQueryPayload {
        max_results: 3,
        role: Some(NodeRole::Relay),
        protocol: Some(Protocol::Udp),
    };
    let resp = build_discovery_response_payload(&store, &query, now);
    assert!(resp.advertisements.len() <= 3);
    assert!(resp.advertisements.iter().all(|a| a.role == NodeRole::Relay));

    // Clamp large max_results to MAX_DISCOVERY_RESPONSE_ADS (32).
    let query2 = DiscoveryQueryPayload {
        max_results: 200,
        role: None,
        protocol: Some(Protocol::Udp),
    };
    let resp2 = build_discovery_response_payload(&store, &query2, now);
    assert!(resp2.advertisements.len() <= 32);
}

