# Contributing to Tactus

Thank you for helping make drumming gear accessible. **One rule shapes everything
here:**

## The one rule: Nonvisual-first

> The nonvisual experience **is** the product. The visual UI is a removable
> enhancement, never a requirement.

This is our North Star ([SPEC](docs/SPEC.md#-north-star--nonvisual-first),
[ADR-0006](docs/adr/0006-nonvisual-first.md)). It is **progressive enhancement
from the nonvisual baseline** — we build for screen-reader/speech/earcon/haptic
use **first**, eyes closed, and only then add visual affordances for sighted
helpers and low-vision users.

**Litmus test for any change:** *"If it can't be done eyes-closed, it isn't
done."*

## Required workflow for every feature / PR

1. **Design the nonvisual interaction first.** What is spoken? Which standard
   screen-reader gestures? What earcons/haptics? What does the read-back say after
   an edit?
2. **Build it nonvisually.** Standard native controls with correct
   label / value / role; announcements for dynamic changes (throttled).
3. **Test eyes-closed — this is the merge gate:**
   - Automated nonvisual assertions (labels/values/roles/announcements) pass.
   - You personally operated the change with **VoiceOver** *and* **TalkBack**,
     screen ignored.
4. **Only then** add/refine the **visual** layer (and confirm the app still works
   with it removed/screen off).
5. Tests: nonvisual assertions are **primary and written first**; visual checks
   are secondary (see [SPEC §16](docs/SPEC.md#16-testing-strategy)).

A PR that adds functionality usable only with sight is **incomplete**, not a
separate "accessibility follow-up."

## PR checklist (must pass before merge)

Copy this into your PR description and tick it:

- [ ] **Nonvisual-first:** the change is fully usable **eyes-closed**; the visual
      layer is enhancement only and can be removed without losing function.
- [ ] Every control has a correct **label, value, and role**.
- [ ] No information conveyed by **colour / position / icon alone**.
- [ ] Adjustable values use the platform **adjustable** pattern (swipe up/down).
- [ ] Focus order logical; headings present; focus handled on navigation.
- [ ] Dynamic changes **announced** — and **throttled** (no spam, no overlap).
- [ ] Targets ≥ **44 pt / 48 dp**; Dynamic Type & high-contrast don't break it.
- [ ] Errors are **spoken and actionable**; no silent failures.
- [ ] Speaks the **read-back** value, never the intended value (no blind writes).
- [ ] **Verified with VoiceOver and TalkBack, eyes closed.**

(Full guidance: [ACCESSIBILITY.md](docs/ACCESSIBILITY.md).)

## Other ground rules

- **No Roland documents in commits.** The Roland PDFs are © Roland and are
  git-ignored; commit only our own derived data/notes, and cite the source
  ([ADR-0004](docs/adr/0004-vendor-docs-not-committed.md),
  [docs/vendor/README.md](docs/vendor/README.md)).
- **Shared logic goes in the Rust core**, not duplicated per platform
  ([ADR-0002](docs/adr/0002-rust-core-uniffi.md)). UI stays native and thin
  ([ADR-0005](docs/adr/0005-native-ui-per-platform.md)).
- Keep the two platforms behaviourally consistent (shared core + shared design
  spec).

By contributing, you agree your contributions are licensed under
[Apache-2.0](LICENSE).
