//! Cassette golden-replay: feed a recorded session's host→device messages through a
//! [`VirtualDevice`] and assert it reproduces the recorded device→host bytes.
//!
//! Today the cassettes are synthetic, so this primarily pins the replay machinery
//! (parse → group → replay → byte-compare) and regression-locks the simulator's
//! wire output. Once a real V31 session is captured (Phase 3), the *same* test
//! becomes a model-vs-reality oracle: any divergence between the hand-built model
//! and the hardware fails here, at the exact timestamp.

use devicesim::{Cassette, CassetteEvent, CassetteMeta, Direction, VirtualDevice, bytes_to_hex};
use sysex::{build_identity_request, build_rq1};

/// A committed fixture (the identity handshake) loaded from the workspace tree —
/// exercises the real on-disk format + the `WORKSPACE_DIR` load path with bytes
/// authored independently of the simulator.
const V31_IDENTITY: &str = include_str!(concat!(
    env!("WORKSPACE_DIR"),
    "/tools/cassettes/v31-identity.ndjson"
));

/// Replay each exchange's request through a fresh device and assert its replies
/// match the recorded responses, byte-for-byte.
fn assert_reproduces(cassette: &Cassette, device: &mut VirtualDevice) {
    for exchange in cassette.exchanges() {
        let request = exchange.request.bytes().expect("valid request hex");
        let got = device.respond(&request);
        let expected: Vec<Vec<u8>> = exchange
            .responses
            .iter()
            .map(|e| e.bytes().expect("valid response hex"))
            .collect();
        assert_eq!(
            got.iter().map(|b| bytes_to_hex(b)).collect::<Vec<_>>(),
            expected.iter().map(|b| bytes_to_hex(b)).collect::<Vec<_>>(),
            "device diverged from the recording at t_us={}",
            exchange.request.t_us
        );
    }
}

#[test]
fn reproduces_the_committed_identity_handshake() {
    let cassette = Cassette::parse(V31_IDENTITY).expect("fixture parses");
    assert_eq!(cassette.meta.profile_id, "roland-v31");
    assert_reproduces(&cassette, &mut VirtualDevice::v31());
}

/// Build a realistic session (identity + the three reads the engine issues on
/// connect) by driving a device, serialize it to a cassette, parse it back, and
/// confirm a *fresh* device reproduces every recorded reply. This round-trips the
/// whole pipeline (record → NDJSON → parse → replay) without any hand-authored hex.
#[test]
fn records_and_replays_a_generated_connect_session() {
    let model = VirtualDevice::v31().profile().model_id.clone();
    let dev_id = VirtualDevice::v31().device_id();

    // The host→device requests of a connect: identity, then current-kit, kit-name
    // and kit-tempo reads for the active kit (index 4).
    let name_addr = VirtualDevice::v31()
        .profile()
        .address_of("kit.common.name", &[4])
        .unwrap();
    let tempo_addr = VirtualDevice::v31()
        .profile()
        .address_of("kit.common.tempo", &[4])
        .unwrap();
    let cur_addr = VirtualDevice::v31()
        .profile()
        .address_of("current.kit_num", &[])
        .unwrap();
    let requests = [
        (build_identity_request(0x7F), Some("connect")),
        (build_rq1(dev_id, &model, cur_addr, [0, 0, 0, 4]), None),
        (build_rq1(dev_id, &model, name_addr, [0, 0, 0, 16]), None),
        (build_rq1(dev_id, &model, tempo_addr, [0, 0, 0, 4]), None),
    ];

    // Record: drive a device and capture each request and its replies with timings.
    let mut recorder = VirtualDevice::v31();
    let mut events = Vec::new();
    let mut t = 0u64;
    for (request, action) in &requests {
        events.push(CassetteEvent::from_bytes(
            t,
            Direction::Out,
            request,
            *action,
        ));
        for reply in recorder.respond(request) {
            t += 8_000;
            events.push(CassetteEvent::from_bytes(t, Direction::In, &reply, None));
        }
        t += 1_000;
    }
    let cassette = Cassette::new(
        CassetteMeta {
            profile_id: "roland-v31".to_string(),
            firmware: Some("0.2.10".to_string()),
            fw_bytes: Some([0, 2, 1, 0]),
            device_id: Some(dev_id),
            platform: None,
            recorder: Some("synthetic".to_string()),
            notes: Some("generated connect session".to_string()),
        },
        events,
    );

    // It captured four solicited exchanges, each with exactly one reply.
    let reparsed = Cassette::parse(&cassette.to_ndjson()).expect("round-trips through NDJSON");
    assert_eq!(reparsed, cassette);
    let exchanges = reparsed.exchanges();
    assert_eq!(exchanges.len(), 4);
    assert!(exchanges.iter().all(|e| e.responses.len() == 1));

    // A fresh device reproduces the whole recording.
    assert_reproduces(&reparsed, &mut VirtualDevice::v31());
}
