//! Timing-dependent scenarios — the reason the virtual clock exists. None of these
//! is expressible with an immediate, synchronous request→response loop: each turns
//! on *when* a reply lands relative to a tick or another reply.

use e2e::Harness;
use engine::CoreEvent;

/// **Bug B (PROTOCOL §6), reproduced deterministically.** A poll's read-back for
/// `current.kit_num` (address `00 00 00 00`) is in flight when `select_kit` fires;
/// `select_kit` verifies at the *same* address, so the stale poll reply lands on the
/// edit's verify slot first and is read as a mismatch → a spurious failure, before
/// the real kit change is picked up.
///
/// This test pins the **current (buggy)** behaviour so the fix has a baseline to
/// flip. The desired post-fix behaviour is in the `#[ignore]`d test below.
#[test]
fn select_kit_race_currently_emits_a_spurious_failure() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle(); // Ready, kit 4
    h.take_events();

    // A poll goes out for current.kit_num; the device answers "kit 4" now, but the
    // reply is still in flight (latency).
    h.poll();
    // Before it arrives, select kit 0: this overwrites the pending slot at the
    // shared address with an edit-verify, writes kit 0, and sends its own verify.
    h.select_kit(0);
    // Settle: the stale poll reply (kit 4) is delivered first, onto the verify slot.
    h.run_to_idle();

    let events = h.events();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, CoreEvent::EditFailed { .. })),
        "the stale read-back currently produces a spurious EditFailed"
    );
    // The real kit change still happens (the verify reply re-routes as unsolicited).
    assert!(events.iter().any(|e| matches!(e,
        CoreEvent::CurrentKitChanged { number, .. } if *number == 0)));
}

/// The same race, asserting the behaviour we *want*. Un-ignore when `select_kit`
/// confirms via the Current poll rather than the shared-address edit pipeline.
#[test]
#[ignore = "desired post-fix behaviour for bug B (PROTOCOL §6); un-ignore when select_kit no longer verifies at the shared Current address"]
fn select_kit_race_should_not_emit_a_false_failure() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    h.poll();
    h.select_kit(0);
    h.run_to_idle();

    let events = h.events();
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, CoreEvent::EditFailed { .. })),
        "no spurious failure once the race is fixed"
    );
    assert!(events.iter().any(|e| matches!(e,
        CoreEvent::CurrentKitChanged { number, name } if *number == 0 && name == "Rock")));
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

/// While an edit is in flight, a tick must not issue a Current poll — the poll's
/// read-back would clobber the edit's verify slot (the same address).
#[test]
fn poll_is_skipped_while_an_edit_is_in_flight() {
    let mut h = Harness::v31("en");
    h.connect().run_to_idle();
    h.take_events();

    h.select_kit(0); // leaves an edit-verify pending on Current (not yet settled)
    let now = h.now();
    let fx = h.act_capturing(move |s| s.tick(now));
    assert!(
        !fx.iter().any(|e| matches!(e, engine::Effect::SendMidi(_))),
        "poll must be skipped while an edit is in flight"
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
