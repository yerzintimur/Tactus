# ADR-0004: Don't commit Roland's PDFs; ship only our derived data

**Status:** Accepted · **Date:** 2026-06-07

## Context
The project is built against the **V31 MIDI Implementation** and **V31 Data List**
documents, which are **© Roland Corporation**. The project is open-source and will
be published on GitHub.

## Decision
- **Do not commit or redistribute Roland's PDFs.** They are git-ignored in
  `docs/vendor/` (and `reference/`), kept locally for development only.
- **Do commit our own derived data and notes** — a JSON parameter/instrument map
  we generate, and protocol notes in our own words ([PROTOCOL.md](../PROTOCOL.md)).
  Individual facts (an address, a checksum formula, an instrument number) are not
  copyrightable; a freshly authored data file describing them is fine to publish.
- **Always cite the source** (document title + version + date) next to derived
  data, and tell users where to download the originals
  ([vendor/README.md](../vendor/README.md)).
- Carry a **trademark/affiliation disclaimer** in `NOTICE`.

## Rationale
Redistributing a third party's copyrighted manuals — even verbatim tables — is
infringement. Re-expressing the underlying facts in our own representation is
both legally cleaner and more useful (typed, versioned, machine-readable).

> Engineering guideline, not legal advice. When unsure about a specific artifact,
> keep it out of the public repo.

## Consequences
- Contributors must download the Roland docs themselves to work on the parser.
- A build step converts the (local) Data List into committed JSON.
- Slightly more friction; correct licensing posture for open source.
