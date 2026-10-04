//! Transmit Edit Data: a knob turned on the module arrives as a DT1 to the
//! parameter's own address, one per step (PROTOCOL §6, captured on the V31
//! 2026-10-04). Every such write is announced with its label, tagged
//! device-initiated, and the value lands in the snapshot where the cache can hold
//! it. Addresses the profile does not describe are ignored, never guessed.

use e2e::Harness;
use engine::{CoreEvent, SpeechCategory, SpeechSource};

/// The `(category, source)` tags of the announcement with exactly `text`.
fn tags_of(events: &[CoreEvent], text: &str) -> Option<(SpeechCategory, SpeechSource)> {
    events.iter().find_map(|e| match e {
        CoreEvent::Speak(s) if s.text == text => Some((s.category, s.source)),
        _ => None,
    })
}

fn display_of(h: &mut Harness, param_id: &str) -> Option<String> {
    h.snapshot()
        .parameters
        .into_iter()
        .find(|p| p.param_id == param_id)
        .and_then(|p| p.display)
}

#[test]
fn a_knob_turned_on_the_panel_is_announced_with_its_label() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle(); // kit 4
    h.take_events();

    h.hardware_edit("kit.common.volume", &[4], 5).run_to_idle();

    assert_eq!(
        tags_of(h.events(), "Kit volume: 0.5 dB"),
        Some((SpeechCategory::ParamEdit, SpeechSource::DeviceInitiated)),
        "got {:?}",
        h.spoken()
    );
    // The device is the source of truth: what it sent is what the UI shows.
    assert_eq!(
        display_of(&mut h, "kit.common.volume"),
        Some("0.5 dB".to_string())
    );
}

#[test]
fn a_panel_sweep_speaks_every_step() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    // One DT1 per step, as the module sends them; the platform lets each step
    // interrupt the last so the drummer hears the value they settle on.
    for raw in [5, 10, 15] {
        h.hardware_edit("kit.common.volume", &[4], raw);
    }
    h.run_to_idle();

    assert_eq!(
        h.spoken(),
        vec![
            "Kit volume: 0.5 dB",
            "Kit volume: 1.0 dB",
            "Kit volume: 1.5 dB"
        ]
    );
}

#[test]
fn a_pad_edit_is_announced_but_not_cached_per_kit() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    // The snare head's layer-A volume turned down to the floor.
    h.hardware_edit("kit.unit.layer.volume", &[4, 0, 1], -601)
        .run_to_idle();

    assert!(
        h.spoken().contains(&"Layer volume: Silent".to_string()),
        "got {:?}",
        h.spoken()
    );
    // The cache is per parameter id, so a per-pad value must not masquerade as
    // "the" layer volume.
    assert_eq!(display_of(&mut h, "kit.unit.layer.volume"), None);
}

#[test]
fn a_catalogued_panel_edit_speaks_the_name() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    h.hardware_edit("kit.fx.type", &[4, 0], 13).run_to_idle();

    assert!(
        h.spoken().contains(&"Effect type: PHASER".to_string()),
        "got {:?}",
        h.spoken()
    );
}

#[test]
fn an_edit_on_another_kit_is_announced_but_does_not_touch_this_kits_values() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle(); // kit 4
    h.take_events();

    h.hardware_edit("kit.common.volume", &[9], 5).run_to_idle();

    assert!(h.spoken().contains(&"Kit volume: 0.5 dB".to_string()));
    assert_eq!(display_of(&mut h, "kit.common.volume"), None);
}

#[test]
fn panel_edits_speak_the_apps_language() {
    let mut h = Harness::v31("ru");
    h.connect().run_to_idle();
    h.take_events();

    h.hardware_edit("kit.common.volume", &[4], -601)
        .run_to_idle();

    assert!(
        h.spoken().contains(&"Громкость кита: Тишина".to_string()),
        "got {:?}",
        h.spoken()
    );
}

#[test]
fn a_write_the_profile_does_not_describe_is_ignored() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    // A pad's Bus Send Head/Rim switch, exactly as the V31 sent it: real on the
    // module, not in the profile.
    h.feed(&[
        0xF0, 0x41, 0x10, 0x01, 0x06, 0x01, 0x12, 0x04, 0x26, 0x11, 0x02, 0x01, 0x42, 0xF7,
    ])
    .run_to_idle();

    assert!(h.spoken().is_empty(), "got {:?}", h.spoken());
    assert!(h.events().is_empty(), "got {:?}", h.events());
}
