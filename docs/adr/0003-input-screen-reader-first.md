# ADR-0003: Screen-reader-first input; voice & big-target modes are secondary

**Status:** Accepted · **Date:** 2026-06-07

## Context
Open question from day one: should input be **voice** or **gestures**? The
developer has not built for blind users before. The usage context is a *drummer*:
loud environment, hands often on sticks, but most editing happens when not
playing.

## Decision
Neither voice nor a custom gesture system is the primary modality. The **primary
input is the platform screen reader** (VoiceOver / TalkBack) driving a **semantic
native UI**. That gives blind users the swipe/double-tap interaction they already
know, for free, with zero learning curve.

Layered on top, later:
- **Performance mode** — a few full-width, no-aim targets (e.g. next/previous
  kit) for quick changes while playing, with earcon + speech feedback.
- **Voice commands** — optional, push-to-talk, on-device, tiny fixed grammar.
  A convenience layer only.
- **Hardware triggers** (footswitch / pad / external controller) — exploratory.

## Rationale
- A custom gesture vocabulary would force relearning and fight the screen reader.
- Voice recognition degrades badly in the loud, mic-polluted drumming
  environment — least reliable exactly when at the kit — so it can't be the
  foundation.
- Most of the app's work (building kits, editing) happens away from the kit, where
  touch + screen reader is ideal.

## Consequences
- We invest in *being perfectly compatible* with VoiceOver/TalkBack, not in novel
  input tech.
- Voice and Performance mode are V1+, scoped as conveniences with fallbacks.
- Full reasoning: [ACCESSIBILITY.md §2](../ACCESSIBILITY.md#2-input-modality--the-real-answer).
