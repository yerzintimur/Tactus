# ADR-0001: Replace the module's screen, don't mirror it

**Status:** Accepted · **Date:** 2026-06-07

## Context
The Roland V31 has a colour screen with no accessibility. The obvious idea —
"read the module's screen aloud" — is impossible: the V31 exposes its **data
model** over MIDI (parameter values), but **not** its screen contents, cursor
position, menu, or navigation state.

## Decision
Do not attempt to reproduce or mirror the module's on-screen UI. Instead, make
the **phone the accessible interface** (screen-reader-first UI + speech) and treat
the **module as a headless sound engine + physical pad surface**, driven over MIDI
SysEx.

## Consequences
- We design our *own* information architecture optimised for non-visual use,
  rather than being constrained by the module's visual menus.
- Some module-screen concepts (cursor, menu paths) simply don't exist in our app.
- We depend entirely on what the MIDI model exposes; anything not in the model
  (e.g. live cursor) is out of scope (SPEC §9.4).
- This is the foundational decision the whole architecture rests on.
