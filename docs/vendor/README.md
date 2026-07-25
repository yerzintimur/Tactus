# Vendor documentation (Roland) — local only, not committed

This folder is where you keep the official Roland reference documents for the
drum modules we support or study. **They are intentionally git-ignored** (see the
repo `.gitignore`) and must **not** be committed or redistributed.

## Why these files are not in the repo

The Roland documents are **© Roland Corporation** (the V31 Data List PDF carries
a `© 2025 Roland Corporation` notice). This project is open-source, and
redistributing a third party's copyrighted manuals — even verbatim tables — is
copyright infringement. So:

- The **PDFs stay out of the repo** (git-ignored here and in `/reference`).
- What the repo *does* ship is our **own derived data and notes**: a JSON
  parameter/instrument map we generate, and protocol notes in our own words
  (`docs/PROTOCOL.md`). Individual facts — a SysEx address, a checksum formula,
  an instrument number — are **not copyrightable** (they are facts, not creative
  expression), so a freshly authored data file describing them is fine to
  publish. We just don't republish Roland's document.
- Always **cite the source** (document title + version + date) next to derived
  data, and tell users **where to get the originals** (below).

> This is a practical engineering guideline, not legal advice. If in doubt about
> a specific artifact, keep it out of the public repo.

## Where to get the official documents

Download them yourself from Roland (free, no account needed). Each module has an
owner's-manuals page at `roland.com/global/support/by_product/<model>/owners_manuals/`
that lists its **Data List** and **MIDI Implementation** PDFs.

For every module, two documents matter to us:

- the **Data List** — instrument list, drum-kit list, FX/ambience types;
- the **MIDI Implementation** — SysEx framing, parameter address map, checksum.

### Roland V31 — the primary target

<https://www.roland.com/global/support/by_product/v31/owners_manuals/>

- **V31 Data List** — built against `eng02`.
- **V31 MIDI Implementation** — built against **v2.00, dated Nov. 11, 2025**
  (`eng01`).

### Roland TD-17 — studied, not yet supported

<https://www.roland.com/global/support/by_product/td-17/owners_manuals/> — see
[docs/devices/roland-td-17.md](../devices/roland-td-17.md) for what we learned.

- **TD-17 Data List** — `eng03` (© 2022), the revision matching firmware 2.00.
- **TD-17 MIDI Implementation** — **Version 2.00, dated Sep. 1. 2022** (`eng04`).

Take the newest revision Roland offers; the parsers pin golden facts, so a
document revision that moves something will fail loudly rather than silently
produce a wrong map.

## Expected files

```
docs/vendor/
├── README.md                           ← this file (committed)
├── V31_DataList_eng02_W.pdf            ← git-ignored
├── V31_MIDI_Implementation_eng01_W.pdf ← git-ignored
├── TD-17_DataList_eng03_W.pdf          ← git-ignored
└── TD-17_MIDI_Imple_eng04_W.pdf        ← git-ignored
```
