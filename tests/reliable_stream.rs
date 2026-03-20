use vpnnode::stream_reliable::ReliableStream;

#[test]
fn out_of_order_delivery_is_reordered() {
    let mut sender = ReliableStream::new();
    let mut receiver = ReliableStream::new();

    let f1 = sender.build_outgoing_frame(1, b"one".to_vec());
    let f2 = sender.build_outgoing_frame(1, b"two".to_vec());
    let f3 = sender.build_outgoing_frame(1, b"three".to_vec());

    let (d1, _, _) = receiver.process_incoming(&f1);
    assert_eq!(d1, vec![b"one".to_vec()]);

    let (d3, _, _) = receiver.process_incoming(&f3);
    assert!(d3.is_empty());

    let (d2, _, _) = receiver.process_incoming(&f2);
    assert_eq!(d2, vec![b"two".to_vec(), b"three".to_vec()]);
}

#[test]
fn retransmit_path_exposes_unacked_frames() {
    let mut s = ReliableStream::new();
    let _f1 = s.build_outgoing_frame(1, b"a".to_vec());
    let _f2 = s.build_outgoing_frame(1, b"b".to_vec());

    let frames = s.unacked_frames(1);
    assert_eq!(frames.len(), 2);
}

#[test]
fn flow_control_limit_inflight() {
    let mut s = ReliableStream::new();
    for _ in 0..70 {
        let _ = s.build_outgoing_frame(1, b"x".to_vec());
    }
    assert!(s.inflight() >= 64);
}

