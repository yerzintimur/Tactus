//! The TD-17 through the simulator: the first profile built without the module
//! in hand (docs/devices/roland-td-17.md). Everything the profile describes is
//! read and announced; nothing is written until a real TD-17 has confirmed the
//! map (ADR-0016). The V31 keeps writing — the gate is per profile, not global.

use e2e::Harness;
use engine::CoreEvent;

/// `F0 41 dev <model id> 12 …` — a DT1 aimed at the TD-17.
fn td17_dt1s(h: &Harness) -> usize {
    h.sent()
        .iter()
        .filter(|m| m.windows(5).any(|w| w == [0x00, 0x00, 0x00, 0x4B, 0x12]))
        .count()
}

fn display_of(h: &mut Harness, param_id: &str) -> Option<String> {
    h.snapshot()
        .parameters
        .into_iter()
        .find(|p| p.param_id == param_id)
        .and_then(|p| p.display)
}

#[test]
fn a_td17_is_recognised_and_announced_as_read_only() {
    let mut h = Harness::td17("en");
    h.connect().run_to_idle();

    let device = h.snapshot().device.expect("identified");
    assert!(device.recognized);
    assert!(!device.verified);
    assert_eq!(device.name, "Roland TD-17");
    assert_eq!(device.model_id, vec![0, 0, 0, 0x4B]);
    // The Identity Reply carries a code, not the version: code 2 is "2.00".
    assert_eq!(device.firmware, "2.00");

    let spoken = h.spoken();
    assert_eq!(
        spoken.first().map(String::as_str),
        Some(
            "Connected to Roland TD-17, firmware 2.00. The Roland TD-17 profile hasn't been \
             checked on a real module yet: Tactus will read and announce, but change nothing."
        ),
        "got {spoken:?}"
    );
}

#[test]
fn reads_work_the_kit_is_named() {
    let mut h = Harness::td17("en");
    h.connect().run_to_idle();

    assert!(
        h.spoken().contains(&"Kit 1: Studio".to_string()),
        "got {:?}",
        h.spoken()
    );
    assert_eq!(
        display_of(&mut h, "kit.common.name"),
        Some("Studio".to_string())
    );
}

#[test]
fn no_write_leaves_the_app() {
    let mut h = Harness::td17("en");
    h.connect().run_to_idle();
    h.take_events();
    let before = td17_dt1s(&h);

    h.set_parameter("kit.common.volume", vec![0], 5)
        .run_to_idle();
    h.rename_kit(0, "Mine").run_to_idle();
    h.select_kit(1).run_to_idle();
    h.next_kit().run_to_idle();

    let refusals = h
        .events()
        .iter()
        .filter(|e| matches!(e, CoreEvent::EditFailed { .. }))
        .count();
    assert_eq!(refusals, 4, "got {:?}", h.events());
    assert!(
        h.spoken().iter().all(|line| line
            == "Not changed: the Roland TD-17 profile hasn't been checked on a real module, \
                so Tactus writes nothing to it."),
        "got {:?}",
        h.spoken()
    );
    assert_eq!(td17_dt1s(&h), before, "a DT1 reached the module");
    // The module is where it was.
    assert_eq!(
        display_of(&mut h, "kit.common.name"),
        Some("Studio".to_string())
    );
}

#[test]
fn panel_edits_are_still_heard_with_the_td17s_own_pads() {
    let mut h = Harness::td17("en");
    h.connect().run_to_idle();
    h.take_events();

    // Snare head, Main layer, volume to the floor — the TD-17's two layers are
    // Main and Sub, its 20 units start at the kick.
    h.hardware_edit("kit.unit.layer.volume", &[0, 0, 1], -601)
        .run_to_idle();
    h.hardware_edit("kit.unit.layer.volume", &[0, 1, 16], 5)
        .run_to_idle();

    assert_eq!(
        h.spoken(),
        vec![
            "Snare head, Main layer, Layer volume: Silent",
            "Ride edge, Sub layer, Layer volume: 0.5 dB"
        ]
    );
}

#[test]
fn firmware_codes_are_named_as_the_module_shows_them() {
    let mut h = Harness::td17("en");
    h.device_mut().with_firmware([0, 0, 0, 0]);
    h.connect().run_to_idle();
    assert_eq!(
        h.snapshot().device.map(|d| d.firmware),
        Some("1.01 or earlier".to_string())
    );

    // A code the documents do not list is shown as its bytes, never invented.
    let mut h = Harness::td17("en");
    h.device_mut().with_firmware([0, 0, 0, 3]);
    h.connect().run_to_idle();
    assert_eq!(
        h.snapshot().device.map(|d| d.firmware),
        Some("0.0.0.3".to_string())
    );
}

#[test]
fn the_refusal_speaks_the_apps_language() {
    let mut h = Harness::td17("ru");
    h.connect().run_to_idle();
    h.take_events();

    h.set_parameter("kit.common.volume", vec![0], 5)
        .run_to_idle();

    assert!(
        h.spoken().contains(
            &"Не изменено: профиль Roland TD-17 не проверен на реальном модуле, поэтому Tactus \
              в него не пишет."
                .to_string()
        ),
        "got {:?}",
        h.spoken()
    );
}

/// The gate is the profile's, not the app's: the V31 still writes.
#[test]
fn the_verified_v31_still_writes() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    h.set_parameter("kit.common.volume", vec![4], 5)
        .run_to_idle();

    assert!(h.snapshot().device.is_some_and(|d| d.verified));
    assert!(
        h.sent()
            .iter()
            .any(|m| m.windows(4).any(|w| w == [0x01, 0x06, 0x01, 0x12])),
        "no DT1 was sent"
    );
    // A user's own edit is confirmed by its value alone (the screen reader has
    // already said which control they are on).
    assert!(
        h.spoken().contains(&"0.5 dB".to_string()),
        "got {:?}",
        h.spoken()
    );
}
