//! `DeviceProfile::locate` — the inverse of `address_of`, pinned on addresses the
//! V31 actually sent while its panel was edited with Transmit Edit Data on
//! (2026-10-04, kit 10 selected; PROTOCOL §6).

use device::ProfileRegistry;

fn v31() -> device::DeviceProfile {
    ProfileRegistry::with_builtin()
        .match_model(&[1, 6, 1])
        .expect("built-in V31 profile")
        .clone()
}

fn located(address: [u8; 4]) -> Option<(String, Vec<u32>)> {
    let p = v31();
    p.locate(address).map(|l| (l.param.id.clone(), l.indices))
}

#[test]
fn panel_edits_land_on_the_parameters_the_module_named() {
    // Kit volume: `04 24 00 50`, eight steps of 0.5 dB.
    assert_eq!(
        located([0x04, 0x24, 0x00, 0x50]),
        Some(("kit.common.volume".to_string(), vec![9]))
    );
    // The snare's layer-A volume: the module sent head and rim together,
    // units 1 and 2 of layer 0.
    assert_eq!(
        located([0x04, 0x24, 0x52, 0x09]),
        Some(("kit.unit.layer.volume".to_string(), vec![9, 0, 1]))
    );
    assert_eq!(
        located([0x04, 0x24, 0x54, 0x09]),
        Some(("kit.unit.layer.volume".to_string(), vec![9, 0, 2]))
    );
    // Reverb level.
    assert_eq!(
        located([0x04, 0x26, 0x70, 0x02]),
        Some(("kit.reverb.level".to_string(), vec![9]))
    );
    // The current kit number itself has no index.
    assert_eq!(
        located([0, 0, 0, 0]),
        Some(("current.kit_num".to_string(), vec![]))
    );
}

#[test]
fn locate_inverts_address_of_across_the_whole_grid() {
    let p = v31();
    for (id, indices) in [
        ("kit.common.tempo", vec![199]),
        ("kit.unit.layer.instrument", vec![9, 2, 27]),
        ("kit.unit.common.room_send", vec![0, 5]),
        ("kit.pad.pan", vec![42, 13]),
        ("kit.fx.type", vec![4, 3]),
        ("setlist.step", vec![31, 2]),
        ("setlist.name", vec![0]),
    ] {
        let address = p.address_of(id, &indices).expect(id);
        let found = p.locate(address).expect(id);
        assert_eq!(
            (found.param.id.as_str(), found.indices),
            (id, indices),
            "{id}"
        );
    }
}

#[test]
fn addresses_the_profile_does_not_describe_are_not_guessed() {
    // One byte into the kit volume — not where any parameter starts.
    assert_eq!(located([0x04, 0x24, 0x00, 0x51]), None);
    // A pad's Bus Send Head/Rim switch — real on the module, not in the profile.
    assert_eq!(located([0x04, 0x26, 0x11, 0x02]), None);
    // Past the last kit.
    let p = v31();
    let beyond = sysex::address::with_stride([4, 0, 0, 0], [0, 4, 0, 0], 200);
    assert!(
        p.locate(sysex::address::add_offset(beyond, &[0, 0x50]))
            .is_none()
    );
}
