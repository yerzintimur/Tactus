# Accessibility guide (read this first if you've never built for blind users)

This is the most important document in the repo. The whole product is an
accessibility tool; if the accessibility is wrong, nothing else matters. It is
written for a developer who has **never built for blind users before**, so it
starts from the mental model and gets concrete.

> **This doc operationalises the project's North Star, [Nonvisual-first](SPEC.md#-north-star--nonvisual-first).**
> The nonvisual experience is the product; the visual UI is a removable
> enhancement. For every feature the order is: design nonvisual → build nonvisual
> → test eyes-closed → *then* add visual. *"If it can't be done eyes-closed, it
> isn't done."* The merge gate that enforces this is in
> [CONTRIBUTING.md](../CONTRIBUTING.md).

---

## 1. How blind users actually use a phone (the mental model)

A blind person does **not** use a special phone. They use a normal iPhone or
Android phone with the built-in **screen reader** turned on:

- **iOS → VoiceOver.** **Android → TalkBack.**
- The screen reader speaks whatever element is "focused," and the user moves
  focus with **gestures they already know cold**:
  - **Swipe right / left** = move to next / previous element.
  - **Double-tap (anywhere)** = activate the focused element.
  - **Explore by touch** = drag a finger; it reads what's under it.
  - Plus rotor (iOS) / reading controls (Android), two/three-finger gestures for
    scroll, etc.
- Crucially: **the user is an expert in these gestures across every app.** They
  are faster than you'd believe — often running speech at 1.5–2× or higher.

**The single most important implication for us:** if you build the UI out of
**standard, semantic native controls** (real buttons, lists, switches, sliders,
text fields), the screen reader works *for free* and the user is instantly at
home. Most "accessibility work" is not adding features — it's **not breaking**
what the platform gives you, and labelling things correctly.

The fastest way to make an app *unusable* for a blind person is to draw your own
custom controls (custom-rendered buttons, gesture canvases, unlabelled icons)
that the screen reader can't interpret.

---

## 2. Input modality — the real answer

You asked: **voice or gestures?** Here is the honest expert answer.

### It's a false binary. The primary input is the OS screen reader.

- **"Gestures"**, for a blind user, *already means* the VoiceOver/TalkBack swipe
  + double-tap vocabulary. You do **not** design your own gesture language — that
  would force the user to relearn interaction and would conflict with the screen
  reader. You get the right "gestures" automatically by using semantic controls.
- **"Voice"** means voice *commands*. Two flavours: the OS-level voice control
  (free if your UI is accessible) and in-app custom commands you build.

So the real decision isn't "voice **or** gestures." It's: **build a perfectly
screen-reader-compatible UI as the foundation, then add voice as a convenience
layer** for specific moments.

### Why screen-reader-first is the foundation

1. It's what the user already knows — zero learning curve.
2. It's reliable, private, silent, and works in any environment.
3. Most of our app's work — building kits, editing settings — happens **when the
   user is not playing**, sitting with the phone. Touch + screen reader is ideal
   there.

### Where voice fits (and its hard limits)

- The few things needed **while at the kit** are quick: "what kit am I on?",
  "next kit", "what's the tempo?".
- **Drums are loud.** Speech recognition degrades badly in noise, and the mic
  picks up the kit. Voice is *least* reliable exactly when the player is at the
  kit. Treat it as a **convenience layer**, never the foundation:
  - On-device recognition (offline, low latency, private).
  - **Push-to-talk** (a big button) or a wake word — not always-listening.
  - A **tiny fixed grammar** ("next kit", "previous kit", "what kit", "tempo"),
    not free-form dictation.
  - Always confirmable by audio, and always with a non-voice fallback.

### The "hands are on the sticks" problem → Performance mode

While playing, both touch precision and voice are compromised. Solutions, in
order of robustness:

1. **Big no-aim targets.** A "Performance mode" screen with one or two
   full-width buttons (e.g. top half = next kit, bottom half = previous) so the
   user can change kits with a slap, no aiming, while the app **announces + earcons**
   the result. This needs no new skills and works in noise.
2. **Voice command** (push-to-talk) as an alternative.
3. **Hardware (future):** a footswitch, a dedicated pad hit, or an external MIDI
   controller as a physical "next kit" surface.

### Recommendation

- **MVP:** screen-reader-first accessible UI + rich audio output. No custom
  input modality at all.
- **V1:** add Performance mode (big targets) and optional push-to-talk voice
  commands.
- **Later:** explore hardware triggers.

This is captured as [ADR-0003](adr/0003-input-screen-reader-first.md).

---

## 3. Concrete requirements (what "accessible" means in code)

### Labels, roles, values, hints (the core of it)
- Every interactive element exposes:
  - **Label** — what it is ("Tempo").
  - **Value** — current state ("120.0 BPM"). For our parameters this comes from
    the Rust core's `format()`.
  - **Role/trait** — button / adjustable / switch, so the screen reader announces
    how to use it and the right gestures work.
  - **Hint** (optional) — how to use it ("swipe up or down to change").
- iOS: `accessibilityLabel` / `accessibilityValue` / `accessibilityTraits` /
  `accessibilityHint`; SwiftUI `.accessibilityLabel/Value/Hint`,
  `.accessibilityElement`. Android: `contentDescription`, `stateDescription`,
  proper roles via Compose `semantics { }` / `Modifier.semantics`.
- **Adjustable values** (tempo, volume, pan): use the platform "adjustable"
  pattern (iOS `.adjustable` + `accessibilityIncrement/Decrement`; Android
  `RangeInfo` / accessibility actions) so the user changes them with
  swipe-up/down — *not* a tiny draggable slider thumb.

### Don't rely on sight-only cues
- No meaning by **colour alone**, position alone, or **unlabelled icons**. Every
  icon button needs a label. Status conveyed by colour must also be in text/audio.

### Focus & structure
- Logical reading/focus order; group related controls; use headings so the user
  can jump by heading.
- When a screen opens, move focus somewhere sensible and announce context.
- Don't trap focus; don't auto-move focus out from under the user.

### Dynamic changes → announcements (used carefully)
- For things that change without user action (kit changed on the hardware, edit
  confirmed), post an **accessibility announcement** (iOS
  `UIAccessibility.post(.announcement,)` / Android live region or
  `announceForAccessibility`). **But throttle** — see §4; uncontrolled
  announcements are a top complaint.

### Touch targets & low-vision
- Minimum target size **44×44 pt (iOS) / 48×48 dp (Android)**; bigger in
  Performance mode.
- Support **Dynamic Type / font scaling**, **high-contrast** and **dark mode**,
  and don't break layout when text is huge — many users are *low-vision*, not
  fully blind, and use both visuals and audio.
- Respect **Reduce Motion**.

### Timing & errors
- No time-limited interactions; if unavoidable, make them generous/adjustable.
- Every error is **spoken and actionable** ("not connected — plug in the USB
  cable"), never a silent red border.

### Honesty (domain-specific, but an a11y issue here)
- **No blind writes.** Always speak the **read-back** value, never the intended
  one (SPEC §10). A blind user cannot glance to double-check; the app's word is
  the only feedback.
- Confirm destructive actions (overwriting a kit slot) and **say what will be
  lost**.

---

## 4. Audio output: coexisting with the screen reader

This is subtle and easy to get wrong. The app must surface live events (kit
changed, edit confirmed) **even when the user isn't touching the screen** — and it
does so through the screen reader's own announcement channel, never a second voice.
Rules:

- **UI-driven feedback** (the user focused/activated something): let the **screen
  reader** say it (via labels/values/announcements). Don't run your own TTS over
  it — that causes double-talk.
- **Live/ambient feedback** (hardware edits, kit changes from the module): post an
  **accessibility announcement** (iOS `UIAccessibility.post(.announcement)` /
  Android `announceForAccessibility`) so the screen reader serializes it into the
  user's own voice — we never run a synthesizer of our own.
  - **Never overlap.** Coalesce duplicates; let high-priority messages (kit
    changed) **preempt** stale ones (interrupting), so a fast scroll leaves you
    the kit you settled on.
- **Earcons first, speech second.** Play a short distinct tone immediately
  (connected / disconnected / kit-changed / confirmed / error), *then* the slower
  spoken detail. Mid-performance the tone alone may be all the user needs.
- **Haptics** as a silent third channel for confirm/deny.
- **The user's settings win automatically:** because speech goes through the
  screen reader, the user's chosen voice, **rate**, verbosity, and volume are
  honoured — we never force our own.
- **Performance mode** = terse announcements + earcons, to stay out of the way.

### Mixed-language speech (the spoken sentence is localized, the kit name isn't)

The sentence is in the user's language ("Кит 5: …") but device-sourced names
(kits, instruments) are usually **English ASCII** ("Jazz Funk"). One voice reading
both mispronounces the foreign part. Fix = **per-segment language tagging**
(ADR-0011):

- **Screen reader (the only path):** build an attributed accessibility label and
  tag the foreign range with its language — iOS
  `.accessibilitySpeechLanguage = "en-US"` on the kit-name range; Android
  `LocaleSpan(Locale.ENGLISH)`. VoiceOver/TalkBack then voice "Кит 5:" in Russian
  and "Jazz" in English. (Verify the Android `LocaleSpan` path on a device.)
- The core supplies the segments: `LocalizedText.spans` marks each run's language;
  device content defaults to `en`.

---

## 5. Onboarding & help (don't assume sight anywhere)

- First-run must be fully operable with the screen reader from the very first
  screen (no "tap the glowing button" with no label).
- Provide an **audio-first quick start** and a discoverable list of any voice
  commands and gestures the app *adds* (kept minimal).
- Explain the **"changed vs saved"** distinction up front once persistence is
  known (SPEC §14).

---

## 6. Testing (automated is necessary but NOT sufficient)

1. **Automated audits** catch the easy stuff (missing labels, small targets, low
   contrast): Android **Accessibility Scanner** + Espresso accessibility checks;
   iOS **XCUITest accessibility audit** (`performAccessibilityAudit`).
2. **Manual, screen-reader-on testing is mandatory.** Turn on VoiceOver /
   TalkBack and operate the entire app **without looking at the screen**. Most
   real problems (confusing order, double-talk, unlabelled state, announcement
   spam) only show up this way.
3. **Test with actual blind drummers** before release. We are sighted developers
   guessing; they are the experts. Recruit early, watch, listen, iterate. Budget
   for this — it's not optional polish, it's the core validation.
4. Test **low-vision** paths too: largest font size, high contrast, dark mode.

---

## 7. Quick checklist (PR gate)

- [ ] Every control has a correct label, value, and role.
- [ ] No information conveyed by colour/position/icon alone.
- [ ] Adjustable values use the platform adjustable pattern (swipe up/down).
- [ ] Focus order is logical; headings present; focus handled on navigation.
- [ ] Dynamic changes announced — and **throttled** (no spam, no overlap).
- [ ] Targets ≥ 44pt / 48dp; Dynamic Type & high-contrast don't break layout.
- [ ] Errors are spoken and actionable; no silent failures.
- [ ] Speaks the **read-back** value, never the intended value.
- [ ] Verified with VoiceOver **and** TalkBack, eyes closed.

---

## 8. References

- Apple: *Accessibility on iOS*, VoiceOver, SwiftUI accessibility modifiers,
  Human Interface Guidelines → Accessibility.
- Android: *Build accessible apps*, TalkBack, Jetpack Compose semantics,
  Accessibility Scanner.
- W3C **WCAG 2.2** and **Mobile Accessibility** guidance (principles transfer to
  native apps).
- The single best habit: **use your app with your eyes closed and the screen
  reader on, every day.**
