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
fn unknown_module_is_not_matched() {
    let reg = ProfileRegistry::with_builtin();
    assert!(reg.match_model(&[0x7F, 0x7F, 0x7F]).is_none());
}

#[test]
fn untested_firmware_is_reported_not_blocked() {
    let reg = ProfileRegistry::with_builtin();
    let p = reg.match_model(&[1, 6, 1]).unwrap();
    // tested list is empty until verified on hardware -> Unknown (still usable).
    assert_eq!(
        p.firmware_support(FirmwareVersion::new([1, 0, 0, 0])),
        FirmwareSupport::Unknown
    );
}
