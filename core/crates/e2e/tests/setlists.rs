//! Set lists end to end: reading one out of the module, and the edits that let a
//! drummer arrange kits for a gig — set a step, append, remove, reorder, rename.
//! Every value here comes back from the virtual module, never from intent.

use e2e::Harness;
use engine::{CoreEvent, SetlistView};

/// A V31 with two named kits and set list 1 stepping through them.
fn seeded(locale: &str) -> Harness {
    let mut h = Harness::v31(locale);
    h.device_mut()
        .with_kit(4, "Jazz", 1200)
        .with_kit(0, "Rock", 1400)
        .with_kit(11, "Funk", 1300)
        .with_setlist(0, "Concert", &[4, 0]);
    h.connect().run_to_idle();
    h.take_events();
    h
}

/// Open set list 1 and let the kit names fill in (one per poll, by design).
fn opened(locale: &str) -> Harness {
    let mut h = seeded(locale);
    h.read_setlist(0).run_to_idle();
    h.advance(2000);
    h
}

fn setlist(h: &Harness) -> SetlistView {
    h.snapshot().setlist.expect("a set list is open")
}

/// The kits a set list steps through, as "<number> <name>".
fn steps(view: &SetlistView) -> Vec<String> {
    view.steps
        .iter()
        .map(|k| format!("{} {}", k.display_number, k.name))
        .collect()
}

/// A whole set list is 33 values. Asking for them one at a time would be 33
/// requests back to back, which is exactly the burst Roland's implementation
/// notes warn about — so it is one request for the whole block.
#[test]
fn reading_a_set_list_takes_a_single_request() {
    let mut h = seeded("en");
    let fx = h.act_capturing(|s| s.read_setlist(0));
    let requests = fx
        .iter()
        .filter(|e| matches!(e, engine::Effect::SendMidi(_)))
        .count();
    assert_eq!(requests, 1, "the whole set list is read in one request");

    h.run_to_idle();
    let view = setlist(&h);
    assert_eq!(view.display_number, 1);
    assert_eq!(view.name, "Concert");
    assert_eq!(view.capacity, 32);
    // The kits are there immediately; their names arrive over the next polls.
    assert_eq!(
        view.steps
            .iter()
            .map(|k| k.display_number)
            .collect::<Vec<_>>(),
        vec![5, 1]
    );
}

/// A step is a kit *number* on the wire. A number is not something a blind user
/// can act on, so the names are read too — spread over polls rather than fired
/// off in a burst.
#[test]
fn step_kits_are_named_from_the_module() {
    let h = opened("en");
    assert_eq!(steps(&setlist(&h)), vec!["5 Jazz", "1 Rock"]);
}

/// The list ends at its END terminator — the slots past it are not steps, and
/// reporting them would invent a set list the drummer never built.
#[test]
fn the_list_stops_at_its_terminator() {
    let h = opened("en");
    let view = setlist(&h);
    assert_eq!(view.steps.len(), 2, "two kits, not 32 slots");
    assert_eq!(view.capacity, 32, "…out of the 32 the module holds");
}

#[test]
fn appending_a_kit_extends_the_list_and_keeps_it_terminated() {
    let mut h = opened("en");
    h.append_setlist_step(11).run_to_idle();
    h.advance(2000);

    assert_eq!(steps(&setlist(&h)), vec!["5 Jazz", "1 Rock", "12 Funk"]);
    // Re-reading from the module agrees — the terminator moved with the list.
    h.read_setlist(0).run_to_idle();
    h.advance(2000);
    assert_eq!(steps(&setlist(&h)), vec!["5 Jazz", "1 Rock", "12 Funk"]);
}

#[test]
fn removing_a_step_shifts_the_rest_up() {
    let mut h = opened("en");
    h.append_setlist_step(11).run_to_idle();
    h.advance(2000);

    h.remove_setlist_step(0).run_to_idle();
    h.advance(2000);
    assert_eq!(steps(&setlist(&h)), vec!["1 Rock", "12 Funk"]);

    h.read_setlist(0).run_to_idle();
    h.advance(2000);
    assert_eq!(
        steps(&setlist(&h)),
        vec!["1 Rock", "12 Funk"],
        "the module agrees after the shift"
    );
}

#[test]
fn swapping_two_steps_reorders_the_list() {
    let mut h = opened("en");
    h.swap_setlist_steps(0, 1).run_to_idle();
    h.advance(2000);
    assert_eq!(steps(&setlist(&h)), vec!["1 Rock", "5 Jazz"]);

    h.read_setlist(0).run_to_idle();
    h.advance(2000);
    assert_eq!(steps(&setlist(&h)), vec!["1 Rock", "5 Jazz"]);
}

/// Several writes never go out at once: the next one leaves only when the module
/// has confirmed the previous. A dropped write then stops the sequence instead of
/// silently scrambling the order.
#[test]
fn a_multi_step_edit_sends_one_write_at_a_time() {
    let mut h = opened("en");
    let fx = h.act_capturing(|s| s.swap_setlist_steps(0, 1));
    let writes = fx
        .iter()
        .filter(|e| matches!(e, engine::Effect::SendMidi(_)))
        .count();
    // One write + its read-back verify, and nothing of the second step yet.
    assert_eq!(writes, 2, "only the first step write goes out");

    h.run_to_idle();
    assert_eq!(steps(&setlist(&h)), vec!["1 Rock", "5 Jazz"]);
}

#[test]
fn setting_a_step_to_nothing_ends_the_list_there() {
    let mut h = opened("en");
    h.set_setlist_step(1, None).run_to_idle();
    assert_eq!(steps(&setlist(&h)), vec!["5 Jazz"]);
}

#[test]
fn renaming_a_set_list_is_confirmed_by_read_back() {
    let mut h = opened("en");
    h.rename_setlist("Rehearsal").run_to_idle();

    assert!(h.events().iter().any(|e| matches!(e,
        CoreEvent::EditConfirmed { display, .. } if display == "Rehearsal")));
    // The name survives a fresh read: it really is in the module, and it round
    // trips through the nibble-packed encoding the set-list name uses.
    h.read_setlist(0).run_to_idle();
    assert_eq!(setlist(&h).name, "Rehearsal");
}

/// A set list edited on the module while the app has it open is picked up the
/// same way any hardware-side change is (Transmit Edit Data).
#[test]
fn a_step_changed_on_the_module_reaches_the_open_list() {
    let mut h = opened("en");
    h.take_events();

    h.hardware_edit("setlist.step", &[0, 0], 11).run_to_idle();
    h.advance(2000);

    assert!(
        h.events()
            .iter()
            .any(|e| matches!(e, CoreEvent::SetlistChanged { number: 0 }))
    );
    assert_eq!(steps(&setlist(&h)), vec!["12 Funk", "1 Rock"]);
}

/// Editing needs a list open and read: the operations work off the cached steps,
/// and acting on values we don't have would write a guess to the module.
#[test]
fn edits_without_an_open_list_are_refused() {
    let mut h = seeded("en");

    for effects in [
        h.act_capturing(|s| s.append_setlist_step(3)),
        h.act_capturing(|s| s.swap_setlist_steps(0, 1)),
        h.act_capturing(|s| s.remove_setlist_step(0)),
        h.act_capturing(|s| s.rename_setlist("X".into())),
    ] {
        assert!(
            !effects
                .iter()
                .any(|e| matches!(e, engine::Effect::SendMidi(_))),
            "nothing may be written without a list to write to"
        );
        assert!(
            effects
                .iter()
                .any(|e| matches!(e, engine::Effect::Emit(CoreEvent::EditFailed { .. })))
        );
    }
}

#[test]
fn a_set_list_the_module_does_not_have_is_refused() {
    let mut h = seeded("en");
    // The V31 holds 32 set lists: 0..=31.
    let fx = h.act_capturing(|s| s.read_setlist(32));
    assert!(!fx.iter().any(|e| matches!(e, engine::Effect::SendMidi(_))));
    assert!(fx.iter().any(|e| matches!(e,
        engine::Effect::Emit(CoreEvent::EditFailed { reason, .. }) if reason.contains("range"))));
}

#[test]
fn set_list_edits_speak_the_users_language() {
    let mut h = opened("ru");
    h.take_events();

    // Ending the list speaks the terminator's meaning, not the raw −1.
    h.set_setlist_step(1, None).run_to_idle();
    assert!(h.events().iter().any(|e| matches!(e,
        CoreEvent::Speak(s) if s.text == "Конец сет-листа")));

    h.take_events();
    h.set_setlist_step(1, Some(11)).run_to_idle();
    assert!(
        h.events()
            .iter()
            .any(|e| matches!(e, CoreEvent::Speak(s) if s.text == "Кит 12")),
        "a step speaks the kit number the module shows, counted from 1"
    );
}
