//! The built-in TD-17 profile: the first one written without the module, from
//! the documents and two other projects' hardware-tested maps
//! (docs/devices/roland-td-17.md §8). It is recognised, read and named; it is
//! not written to until a TD-17 has confirmed it (ADR-0016).

use device::{DeviceProfile, DimLabel, FirmwareVersion, ProfileRegistry};

fn td17() -> DeviceProfile {
    ProfileRegistry::with_builtin()
        .match_model(&[0, 0, 0, 0x4B])
        .expect("built-in TD-17 profile")
        .clone()
}

#[test]
fn builtin_td17_loads_with_the_documented_identity() {
    let reg = ProfileRegistry::with_builtin();
    // Identity Reply: Roland 41, family 4B 03, number 00 00 — the fingerprint
    // V-Drum Explorer also matches on (docs/devices/roland-td-17.md §8).
    let by_identity = reg
        .match_identity(0x41, [0x4B, 0x03], [0x00, 0x00])
        .expect("TD-17 by identity");
    assert_eq!(by_identity.profile_id, "roland-td-17");
    assert_eq!(by_identity.display_name, "Roland TD-17");
    // The four-byte Model ID frames every RQ1/DT1; the V31's has three.
    assert_eq!(by_identity.model_id, vec![0, 0, 0, 0x4B]);
    assert_eq!(by_identity.capabilities.kit_count, 100);
    assert_eq!(by_identity.max_kit_number(), Some(99));
}

#[test]
fn addresses_follow_the_documented_layout() {
    let p = td17();
    // Kit 1 at 03 00 00 00, kits 00 02 00 00 apart; the name is 12 characters
    // here (16 on the V31), the sub-name 16 (64).
    assert_eq!(p.address_of("kit.common.name", &[0]), Some([3, 0, 0, 0]));
    assert_eq!(p.address_of("kit.common.name", &[1]), Some([3, 2, 0, 0]));
    assert_eq!(p.parameter("kit.common.name").unwrap().len, 12);
    assert_eq!(p.parameter("kit.common.sub_name").unwrap().len, 16);
    // Kit Unit Main at 00 40 00, Sub at 00 60 00, one unit per 00 01 00:
    // the Sub layer's volume of unit 3 (tom 1 head) in kit 1.
    assert_eq!(
        p.address_of("kit.unit.layer.volume", &[0, 1, 2]),
        Some([3, 0, 0x62, 4])
    );
    // Current kit: one byte, 0–99, shown 1–100.
    let kit_num = p.parameter("current.kit_num").unwrap();
    assert_eq!(kit_num.len, 1);
    assert_eq!(kit_num.display_offset, 1);
    assert_eq!(p.address_of("current.kit_num", &[]), Some([0, 0, 0, 0]));
}

#[test]
fn the_td17_is_unverified_and_the_v31_is_verified() {
    let reg = ProfileRegistry::with_builtin();
    let td17 = reg.match_model(&[0, 0, 0, 0x4B]).unwrap();
    let v31 = reg.match_model(&[1, 6, 1]).unwrap();
    assert!(!td17.allows_writes());
    assert!(
        td17.verification.basis.is_some(),
        "say what the map rests on"
    );
    assert!(v31.allows_writes());
    assert!(v31.verification.basis.is_some());
}

/// The Identity Reply's revision is a code; the profile names it as the module
/// and its manual do (docs/devices/roland-td-17.md §3).
#[test]
fn firmware_codes_are_named() {
    let p = td17();
    let shown = |b: [u8; 4]| p.firmware.display(FirmwareVersion::new(b));
    assert_eq!(shown([0, 0, 0, 0]), "1.01 or earlier");
    assert_eq!(shown([0, 0, 0, 1]), "1.02");
    assert_eq!(shown([0, 0, 0, 2]), "2.00");
    // A code the documents do not list: the bytes, not a guess.
    assert_eq!(shown([0, 0, 0, 3]), "0.0.0.3");
}

/// Twenty units in the documented order, two layers Main/Sub; the ride's second
/// zone is RIDE RIM in this document and RIDE EDGE in the V31's — one key.
#[test]
fn units_and_layers_are_named() {
    let p = td17();
    let volume = p.parameter("kit.unit.layer.volume").unwrap();
    let unit = volume.dims.iter().find(|d| d.name == "unit").unwrap();
    let layer = volume.dims.iter().find(|d| d.name == "layer").unwrap();
    assert_eq!(unit.count, 20);
    assert_eq!(p.dim_label(unit, 0), Some(DimLabel::Named("pad.kick")));
    assert_eq!(
        p.dim_label(unit, 1),
        Some(DimLabel::Named("pad.snare.head"))
    );
    assert_eq!(
        p.dim_label(unit, 16),
        Some(DimLabel::Named("pad.ride.edge"))
    );
    assert_eq!(p.dim_label(unit, 19), Some(DimLabel::Named("pad.aux.rim")));
    assert_eq!(p.dim_label(unit, 20), None);
    assert_eq!(layer.count, 2);
    assert_eq!(p.dim_label(layer, 0), Some(DimLabel::Named("layer.main")));
    assert_eq!(p.dim_label(layer, 1), Some(DimLabel::Named("layer.sub")));
}

/// Every level is a dB value with the family-wide floor: −601 is −INF on the
/// TD-17 exactly as on the V31 (V-Drum Explorer's `volume32`).
#[test]
fn levels_share_the_family_floor() {
    let p = td17();
    let levels: Vec<_> = p
        .parameters
        .iter()
        .filter(|d| d.unit.as_deref() == Some("db"))
        .collect();
    assert!(levels.len() >= 8, "found {}", levels.len());
    for def in levels {
        let range = def.range.expect("a level has a range");
        assert_eq!(range.min, -601, "{}", def.id);
        assert_eq!(
            def.sentinel.as_ref().map(|s| s.raw),
            Some(-601),
            "{}: the floor must be named, not spoken as -60.1 dB",
            def.id
        );
    }
}
