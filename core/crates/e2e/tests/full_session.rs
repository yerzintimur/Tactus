//! End-to-end behaviour of the session against a virtual V31: connect → identify →
//! poll → edit → verify, kit navigation, renames, hardware pushes, and the pull-side
//! snapshot. These pin the same public contract the engine's old inline tests did,
//! but through the timing-aware [`Harness`] driving a [`devicesim::VirtualDevice`].

use e2e::Harness;
use engine::{ConnectionState, CoreEvent, ParamKind, ParamValue, SpeechPriority};

fn has_speak(events: &[CoreEvent], text: &str) -> bool {
    events
        .iter()
        .any(|e| matches!(e, CoreEvent::Speak(s) if s.text == text))
}

#[test]
fn connect_identify_and_read_current_kit() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    let events = h.events();

    assert!(events.iter().any(|e| matches!(e,
        CoreEvent::DeviceIdentified(d) if d.recognized && d.name == "Roland V31")));
    assert!(events.iter().any(|e| matches!(e,
        CoreEvent::CurrentKitChanged { number, name } if *number == 4 && name == "Jazz")));
    // The connect line is always High priority and names the module.
    assert!(events.iter().any(|e| matches!(e,
        CoreEvent::Speak(s) if s.text.contains("Roland V31") && s.priority == SpeechPriority::High)));
    assert!(has_speak(events, "Kit 5: Jazz"));
    assert!(has_speak(events, "120.0 BPM"));
    assert_eq!(h.state(), ConnectionState::Ready);
}

#[test]
fn speaks_russian_kit_label() {
    let mut h = Harness::v31("ru");
    h.connect().run_to_idle();
    assert!(has_speak(h.events(), "Кит 5: Jazz"));
    assert!(has_speak(h.events(), "120.0 уд/мин"));
}

#[test]
fn unknown_module_degrades_without_crashing() {
    let mut h = Harness::v31("en");
    // Keep the (V31) device quiet so only our crafted foreign reply is processed.
    h.device_mut().set_responsive(false);
    h.connect();
    // Identity Reply for a different Roland device (family 09 09).
    let reply = [
        0xF0, 0x7E, 0x10, 0x06, 0x02, 0x41, 0x09, 0x09, 0x00, 0x00, 0, 0, 0, 0, 0xF7,
    ];
    h.feed(&reply);
    assert!(h.events().iter().any(|e| matches!(e,
        CoreEvent::DeviceIdentified(d) if !d.recognized)));
    assert_eq!(h.state(), ConnectionState::Ready);
}

#[test]
fn hardware_kit_change_is_picked_up() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle(); // Ready, kit 4
    h.take_events();

    // The module pushes an unsolicited Current change to kit index 0.
    h.hardware_select_kit(0).run_to_idle();
    assert!(
        h.events()
            .iter()
            .any(|e| matches!(e, CoreEvent::Earcon(engine::Earcon::KitChanged)))
    );
    assert!(has_speak(h.events(), "Kit 1: Rock"));
}

#[test]
fn set_parameter_confirmed_by_readback() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    h.set_parameter("kit.common.tempo", vec![4], 1300)
        .run_to_idle();
    assert!(h.events().iter().any(|e| matches!(e,
        CoreEvent::EditConfirmed { display, .. } if display == "130.0 BPM")));
    assert!(has_speak(h.events(), "130.0 BPM"));
}

#[test]
fn edit_mismatch_announces_the_actual_value() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    // The module acknowledges but does not apply the write: read-back still 1200.
    h.device_mut().reject_next_write();
    h.set_parameter("kit.common.tempo", vec![4], 1300)
        .run_to_idle();

    assert!(
        h.events()
            .iter()
            .any(|e| matches!(e, CoreEvent::EditFailed { .. }))
    );
    // Announces the TRUTH (still 120.0 BPM), never the intended 130.0.
    assert!(
        h.events().iter().any(|e| matches!(e,
        CoreEvent::Speak(s) if s.text.contains("120.0 BPM") && s.priority == SpeechPriority::High))
    );
}

#[test]
fn out_of_range_edit_is_rejected_without_sending() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    // 999999 needs more than 4 nibbles (max 65535) -> rejected before any I/O: the
    // edit produces only the failure events, no MIDI and no scheduled tick.
    let fx = h.act_capturing(|s| s.set_parameter("kit.common.tempo".into(), vec![4], 999_999));
    assert!(!fx.iter().any(|e| matches!(e, engine::Effect::SendMidi(_))));
    assert!(
        !fx.iter()
            .any(|e| matches!(e, engine::Effect::ScheduleTick { .. }))
    );
    assert!(fx.iter().any(|e| matches!(e,
        engine::Effect::Emit(CoreEvent::EditFailed { reason, .. }) if reason.contains("range"))));
}

#[test]
fn select_kit_confirmed_reads_the_new_kit() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle(); // Ready on kit 4 ("Jazz")
    h.take_events();

    h.select_kit(0).run_to_idle(); // switch to kit index 0 ("Rock")
    assert!(h.events().iter().any(|e| matches!(e,
        CoreEvent::CurrentKitChanged { number, name } if *number == 0 && name == "Rock")));
    assert!(has_speak(h.events(), "Kit 1: Rock"));
}

#[test]
fn rename_kit_confirmed_by_readback() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    h.rename_kit(4, "Funk").run_to_idle();
    assert!(h.events().iter().any(|e| matches!(e,
        CoreEvent::EditConfirmed { display, .. } if display == "Funk")));
}

#[test]
fn disconnect_resets_and_earcons() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    h.disconnect();
    assert_eq!(h.state(), ConnectionState::Disconnected);
    assert!(
        h.events()
            .iter()
            .any(|e| matches!(e, CoreEvent::Earcon(engine::Earcon::Disconnected)))
    );
}

#[test]
fn snapshot_before_connect_is_disconnected_and_empty() {
    let h = Harness::v31("en");
    let snap = h.snapshot();
    assert_eq!(snap.connection, ConnectionState::Disconnected);
    assert!(snap.device.is_none());
    assert!(snap.current_kit.is_none());
    assert!(snap.parameters.is_empty());
}

#[test]
fn snapshot_reports_device_kit_and_parameter_metadata() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle(); // Ready, kit 4 = "Jazz", tempo 1200
    let snap = h.snapshot();

    assert_eq!(snap.connection, ConnectionState::Ready);
    assert!(
        snap.device
            .as_ref()
            .is_some_and(|d| d.recognized && d.name == "Roland V31")
    );

    let kit = snap.current_kit.expect("current kit known");
    assert_eq!(
        (kit.number, kit.display_number, kit.name.as_str()),
        (4, 5, "Jazz")
    );

    let tempo = snap
        .parameters
        .iter()
        .find(|p| p.param_id == "kit.common.tempo")
        .expect("tempo view");
    assert_eq!(tempo.label, "Tempo");
    assert_eq!(tempo.kind, ParamKind::Numeric);
    assert_eq!(tempo.value, Some(ParamValue::Int(1200)));
    assert_eq!(tempo.display.as_deref(), Some("120.0 BPM"));
    let range = tempo
        .numeric
        .as_ref()
        .and_then(|n| n.range.as_ref())
        .expect("range");
    assert_eq!(
        (range.raw_min, range.raw_max, range.raw_step),
        (200, 2600, 1)
    );

    let name = snap
        .parameters
        .iter()
        .find(|p| p.param_id == "kit.common.name")
        .expect("name view");
    assert_eq!(name.kind, ParamKind::Text);
    assert_eq!(name.value, Some(ParamValue::Text("Jazz".to_string())));
    assert!(name.numeric.is_none());
}

#[test]
fn snapshot_reflects_a_confirmed_edit() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.set_parameter("kit.common.tempo", vec![4], 1300)
        .run_to_idle();

    let tempo = h
        .snapshot()
        .parameters
        .into_iter()
        .find(|p| p.param_id == "kit.common.tempo")
        .unwrap();
    assert_eq!(tempo.value, Some(ParamValue::Int(1300)));
    assert_eq!(tempo.display.as_deref(), Some("130.0 BPM"));
}

#[test]
fn snapshot_localizes_labels_and_values() {
    let mut h = Harness::v31("ru");
    h.connect().run_to_idle();

    let tempo = h
        .snapshot()
        .parameters
        .into_iter()
        .find(|p| p.param_id == "kit.common.tempo")
        .unwrap();
    assert_eq!(tempo.label, "Темп");
    assert_eq!(tempo.display.as_deref(), Some("120.0 уд/мин"));
}

#[test]
fn snapshot_clears_stale_values_on_kit_change() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle(); // kit 4, tempo 1200
    h.select_kit(0).run_to_idle(); // switch to kit 0 ("Rock", tempo 1400)

    let snap = h.snapshot();
    assert_eq!(snap.current_kit.unwrap().name, "Rock");
    let tempo = snap
        .parameters
        .into_iter()
        .find(|p| p.param_id == "kit.common.tempo")
        .unwrap();
    assert_eq!(tempo.value, Some(ParamValue::Int(1400)));
}
