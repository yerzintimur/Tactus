//! Black-box (integration) tests: exercise only the public `sysex` API and assert
//! the exact byte strings from the spec's golden vectors (docs/PROTOCOL.md §3).
//! These pin the public wire contract.

use sysex::{SysexMessage, build_dt1, build_identity_request, build_rq1, parse};

const V31_MODEL: [u8; 3] = [0x01, 0x06, 0x01];

/// G1 — DT1 write: SNARE HEAD layer-A EQ of kit 1 = ON.
#[test]
fn dt1_matches_golden_g1() {
    let msg = build_dt1(0x10, &V31_MODEL, [0x04, 0x00, 0x52, 0x21], &[0x01]);
    assert_eq!(
        msg,
        vec![
            0xF0, 0x41, 0x10, 0x01, 0x06, 0x01, 0x12, 0x04, 0x00, 0x52, 0x21, 0x01, 0x08, 0xF7
        ]
    );
}

/// G2 — RQ1 read: snare pad compressor switch of kit 1.
#[test]
fn rq1_matches_golden_g2() {
    let msg = build_rq1(0x10, &V31_MODEL, [0x04, 0x02, 0x11, 0x0D], [0, 0, 0, 1]);
    assert_eq!(
        msg,
        vec![
            0xF0, 0x41, 0x10, 0x01, 0x06, 0x01, 0x11, 0x04, 0x02, 0x11, 0x0D, 0x00, 0x00, 0x00,
            0x01, 0x5B, 0xF7
        ]
    );
}

#[test]
fn identity_request_matches_spec() {
    assert_eq!(
        build_identity_request(0x10),
        vec![0xF0, 0x7E, 0x10, 0x06, 0x01, 0xF7]
    );
}

/// What we build, we can parse back (DT1 round-trip).
#[test]
fn dt1_build_parse_roundtrip() {
    let bytes = build_dt1(0x10, &V31_MODEL, [0x04, 0x00, 0x52, 0x21], &[0x01]);
    assert_eq!(
        parse(&bytes, &V31_MODEL),
        Ok(SysexMessage::Dt1 {
            device_id: 0x10,
            address: [0x04, 0x00, 0x52, 0x21],
            data: vec![0x01],
        })
    );
}
