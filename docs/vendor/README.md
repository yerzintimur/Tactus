# Vendor documentation (Roland) — local only, not committed

This folder is where you keep the official Roland reference documents for the
**V31** drum module. **They are intentionally git-ignored** (see the repo
`.gitignore`) and must **not** be committed or redistributed.

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

Download them yourself from Roland (free, no account needed):

1. Go to <https://www.roland.com/global/support/by_product/v31/owners_manuals/>
   (or search "Roland V31 owner's manuals").
2. Download:
   - **V31 Data List** (instrument list, drum-kit list, FX/ambience types,
     full parameter tables). This repo was built against `eng02`.
   - **V31 MIDI Implementation** (SysEx format, parameter address map, checksum).
     This repo was built against **v2.00, dated Nov. 11, 2025** (`eng01`).
3. Drop the PDFs into this folder. The build/parse scripts and `Read` tooling
   look for them here.

## Expected files

```
docs/vendor/
├── README.md                          ← this file (committed)
├── V31_DataList_eng02_W.pdf           ← git-ignored
└── V31_MIDI_Implementation_eng01_W.pdf← git-ignored
```
