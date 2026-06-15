//! Integration test for the real, embedded V31 profile (`profiles/roland-v31.json`)
//! through the public API. Pins the MVP addresses against docs/PROTOCOL.md.

use device::{FirmwareSupport, FirmwareVersion, ProfileRegistry};

#[test]
fn builtin_v31_loads_and_resolves_mvp_addresses() {
    let reg = ProfileRegistry::with_builtin();
    let p = reg
        .match_model(&[1, 6, 1])
        .expect("V31 auto-detected by Model ID");

    assert_eq!(p.profile_id, "roland-v31");
    assert_eq!(p.capabilities.kit_count, 200);

    // Current kit pointer lives at 00 00 00 00.
    assert_eq!(
        p.address_of("current.kit_num", &[]),
        Some([0x00, 0x00, 0x00, 0x00])
    );

    // Kit name: kit 1 (index 0) at the kit base; kit 200 (index 199) at 0A 1C 00 00.
    assert_eq!(
        p.address_of("kit.common.name", &[0]),
        Some([0x04, 0x00, 0x00, 0x00])
    );
    assert_eq!(
        p.address_of("kit.common.name", &[199]),
        Some([0x0A, 0x1C, 0x00, 0x00])
    );

    // Kit tempo of kit 1: KitCommon offset 0x6C (the #-split nibble field 6C..6F).
    assert_eq!(
        p.address_of("kit.common.tempo", &[0]),
        Some([0x04, 0x00, 0x00, 0x6C])
    );
}

#[test]
fn matches_by_identity_reply_fingerprint() {
    // V31 Identity Reply: manufacturer 0x41, family [01 06], member [03 00].
    let reg = ProfileRegistry::with_builtin();
    assert_eq!(
        reg.match_identity(0x41, [0x01, 0x06], [0x03, 0x00])
            .unwrap()
            .profile_id,
        "roland-v31"
    );
    assert!(
        reg.match_identity(0x41, [0x09, 0x09], [0x00, 0x00])
            .is_none()
    );
}

#[test]
fn unknown_module_is_not_matched() {
    let reg = ProfileRegistry::with_builtin();
    assert!(reg.match_model(&[0x7F, 0x7F, 0x7F]).is_none());
}

#[test]
fn firmware_support_reflects_the_tested_baseline() {
    let reg = ProfileRegistry::with_builtin();
    let p = reg.match_model(&[1, 6, 1]).unwrap();
    // The live-validated firmware (Identity Reply bytes 00 02 01 00 = "0.2.10")
    // is the tested baseline.
    assert_eq!(
        p.firmware_support(FirmwareVersion::new([0, 2, 1, 0])),
        FirmwareSupport::Tested
    );
    // Anything off that baseline is announced but never blocked (ADR-0009).
    assert_eq!(
        p.firmware_support(FirmwareVersion::new([1, 0, 0, 0])),
        FirmwareSupport::UntestedNewer
    );
    assert_eq!(
        p.firmware_support(FirmwareVersion::new([0, 1, 0, 0])),
        FirmwareSupport::UntestedOlder
    );
}
