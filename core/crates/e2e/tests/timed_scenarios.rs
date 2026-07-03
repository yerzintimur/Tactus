//! Timing-dependent scenarios — the reason the virtual clock exists. None of these
//! is expressible with an immediate, synchronous request→response loop: each turns
//! on *when* a reply lands relative to a tick or another reply.

use e2e::Harness;
use engine::CoreEvent;

/// **Bug B (PROTOCOL §6).** A poll's read-back for `current.kit_num` (address
/// `00 00 00 00`) is in flight when `select_kit` fires. The kit number lives at the
/// *same* address the poller reads, so verifying the selection through the
/// address-keyed edit pipeline used to read the stale poll reply (old kit) as a
/// mismatch → a spurious "value unknown" failure. The selection is instead
/// confirmed via the regular `Current` read path, which ignores stale values: the
/// stale reply is dropped and the real change is announced, with no false failure.
#[test]
fn select_kit_race_does_not_emit_a_false_failure() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle(); // Ready, kit 4
    h.take_events();

    // A poll goes out for current.kit_num; the device answers "kit 4" now, but the
    // reply is still in flight (latency).
    h.poll();
    // Before it arrives, select kit 0: writes kit 0 and issues its own Current read.
    h.select_kit(0);
    // Settle: the stale poll reply (kit 4) is delivered first.
    h.run_to_idle();

    let events = h.events();
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, CoreEvent::EditFailed { .. })),
        "a stale in-flight poll reply must not fail the selection"
    );
    assert!(events.iter().any(|e| matches!(e,
        CoreEvent::CurrentKitChanged { number, name } if *number == 0 && name == "Rock")));
}

/// A kit select the module never acts on (it went silent) still reports an audible
/// failure via the tick-driven timeout — never silence, never a false success.
#[test]
fn kit_select_times_out_when_the_module_never_lands() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    h.device_mut().set_responsive(false); // no Current read will come back
    h.select_kit(0);
    h.advance(2000); // > EDIT_TIMEOUT_TICKS ticks

    assert!(
        h.events()
            .iter()
            .any(|e| matches!(e, CoreEvent::EditFailed { .. })),
        "an unconfirmed kit select must fail audibly"
    );
}

/// A kit select whose write the module rejects: every `Current` read keeps
/// returning the old kit (ignored as stale), so the selection times out — an
/// honest failure, not a false kit-change announcement and not silence.
#[test]
fn rejected_kit_select_fails_by_timeout_not_falsely() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    h.device_mut().reject_next_write();
    h.select_kit(0);
    h.advance(2000);

    let events = h.events();
    assert!(
        !events.iter().any(|e| matches!(e,
            CoreEvent::CurrentKitChanged { number, .. } if *number == 0)),
        "the rejected selection must not be announced as a change"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, CoreEvent::EditFailed { .. })),
        "the rejected selection must fail audibly"
    );
}

/// A fast dial through kits (two unsolicited Current pushes closer together than a
/// name read-back) must announce only the kit we settle on — the intermediate
/// kit's stale name read is dropped.
#[test]
fn rapid_kit_scroll_announces_only_the_settled_kit() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();
    h.device_mut().with_kit(1, "Funk", 1300);

    // Two pushes at the same instant: kit 0, then kit 1.
    h.hardware_select_kit(0);
    h.hardware_select_kit(1);
    h.run_to_idle();

    let spoken = h.spoken();
    assert!(
        spoken.iter().any(|s| s == "Kit 2: Funk"),
        "the settled kit is announced; got {spoken:?}"
    );
    assert!(
        !spoken.iter().any(|s| s.contains("Rock")),
        "the stale kit-0 name must be dropped; got {spoken:?}"
    );
}

/// An edit whose read-back never arrives (the module went silent) times out and is
/// reported as failed — driven purely by tick aging on the virtual clock.
#[test]
fn edit_times_out_without_a_readback() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    h.device_mut().set_responsive(false); // no read-back will come back
    h.set_parameter("kit.common.tempo", vec![4], 1300);
    h.advance(2000); // > EDIT_TIMEOUT_TICKS * poll interval

    assert!(
        h.events()
            .iter()
            .any(|e| matches!(e, CoreEvent::EditFailed { .. })),
        "the edit should time out"
    );
}

/// While an edit is in flight, a tick must not issue a Current poll — if the poll
/// surfaced a kit change mid-verify, the kit-change flow would clear the value
/// cache and issue reads around the verify.
#[test]
fn poll_is_skipped_while_an_edit_is_in_flight() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    h.set_parameter("kit.common.tempo", vec![4], 1300); // verify not yet settled
    let now = h.now();
    let fx = h.act_capturing(move |s| s.tick(now));
    assert!(
        !fx.iter().any(|e| matches!(e, engine::Effect::SendMidi(_))),
        "poll must be skipped while an edit is in flight"
    );
}

/// A kit *selection* is the opposite: the `Current` poll is exactly how it gets
/// confirmed, so it must never suppress polling.
#[test]
fn poll_continues_while_a_kit_select_is_in_flight() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    h.select_kit(0); // confirmation read still in flight
    let now = h.now();
    let fx = h.act_capturing(move |s| s.tick(now));
    assert!(
        fx.iter().any(|e| matches!(e, engine::Effect::SendMidi(_))),
        "the poll is the selection's confirmation path — it must keep running"
    );
}

/// A kit changed on the module *without* an unsolicited push (e.g. Transmit Edit
/// Data off) is still picked up by the periodic poll as the clock advances.
#[test]
fn periodic_poll_detects_a_silent_kit_change() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    h.device_mut().set_current_kit(0); // changed, but the module stays quiet
    h.advance(700); // spans more than two 300 ms poll intervals

    assert!(
        h.events().iter().any(|e| matches!(e,
            CoreEvent::CurrentKitChanged { number, .. } if *number == 0)),
        "the periodic poll should detect the change"
    );
}
