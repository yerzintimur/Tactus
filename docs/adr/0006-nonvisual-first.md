# ADR-0006: Nonvisual-first is the core development philosophy

**Status:** Accepted · **Date:** 2026-06-07

## Context
This app exists so that **blind and low-vision drummers** can operate a Roland
V31. The usual industry default is to build a visual UI and then "add
accessibility" — which reliably produces a second-class nonvisual experience,
because accessibility ends up as a retrofit constrained by visual decisions.

We want the opposite as a hard, structural commitment, not an aspiration. We
also chose the *name* of the philosophy deliberately (see Decision): naming it
after the **design constraint** ("nonvisual") rather than a **group of people**
("blind") keeps it respectful and makes it cover blind users, low-vision users,
and situational cases (a sighted drummer mid-song with eyes/hands occupied).

## Decision
**Nonvisual-first** is the foundational principle of the project.

1. The **complete, primary experience is nonvisual** — operable end-to-end with
   the screen reader, speech, earcons, and haptics, **eyes closed**.
2. The **visual UI is a secondary enhancement** for sighted helpers and
   low-vision users. It is **never required** to perform any function. The app
   must remain fully functional with the screen off.
3. This is **progressive enhancement from the nonvisual baseline** — the inverse
   of graceful degradation.
4. **Order of work for every feature:** design nonvisual → build nonvisual →
   test eyes-closed (VoiceOver + TalkBack, automated + manual) → *only then* add
   visual affordances → re-verify the nonvisual path stands alone.
5. **Definition of done:** a feature is not done until usable eyes-closed. A
   visual-only change is incomplete by definition, not a deferred "a11y task."
6. **Tests follow suit:** nonvisual assertions are primary and written first;
   visual checks are secondary (SPEC §16).

Litmus test: *"If it can't be done eyes-closed, it isn't done."*

## Terminology
Chosen term: **"Nonvisual-first."** Considered and not chosen as the label:
- *Blind-first* — accurate and "blind" is accepted identity-first language, but
  names a group rather than the design constraint and omits low-vision/situational.
- *Screen-reader-first* — good developer framing, used in practice, but narrower
  (e.g. doesn't cover a pure audio mode without a screen reader).
- *Accessibility-first* — too vague; doesn't capture the nonvisual-baseline +
  enhancement inversion.

## Consequences
- **Enforced in process:** the contributor gate ([CONTRIBUTING.md](../../CONTRIBUTING.md))
  and the PR checklist ([ACCESSIBILITY.md](../ACCESSIBILITY.md)) require the
  eyes-closed path before merge.
- Some work is slower up front (you can't ship a visual stub and "do a11y later").
- The payoff: the nonvisual experience is the real product, not a degraded mode.
- Reinforces, and is reinforced by, [ADR-0003](0003-input-screen-reader-first.md)
  (screen-reader-first input) and [ADR-0005](0005-native-ui-per-platform.md)
  (native UI for the best screen-reader support).
