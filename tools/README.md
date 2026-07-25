# tools/ — the data pipeline

Dev-time parsers that turn Roland's PDFs into the **committed derived data** the
core is built on. The PDFs themselves are © Roland and deliberately **not** in git
([ADR-0004](../docs/adr/0004-vendor-docs-not-committed.md)) — download them into
`docs/vendor/` first (see [docs/vendor/README.md](../docs/vendor/README.md)).

| Script | Input (git-ignored) | Output (committed) |
|---|---|---|
| `parse_midi_impl.py` | a MIDI Implementation PDF, §3 "Parameter Address Map" | [profiles/maps/](../profiles/maps/) — every block, parameter offset, size, encoding, range, and value-label list, plus the pad/bus assignment tables |
| `parse_datalist.py` | V31 Data List PDF (+ the address map, for the FX enum) | [profiles/catalogs/roland-v31/](../profiles/catalogs/roland-v31/) — `drum-kits.json` (200 kits), `instruments.json` (preset + EXV expansion packs, with group and remark flags), `fx-types.json` (95 Bus FX types) |

**`parse_midi_impl.py` is device-agnostic.** The table grammar belongs to Roland,
not to one module, so a new module contributes an entry in the script's `DEVICES`
table — paths, citation, and the golden facts to assert — and no parsing code.
Today it derives the V31 (5 areas, 25 blocks, 792 parameters) and the TD-17
(4 areas, 17 blocks, 163 parameters plus 482 in variant tables); run one with
`--device td-17`, or all of them with no arguments. `parse_datalist.py` is **not**
device-agnostic yet: it reads glyph positions tuned to the V31's page layout.

## Running

One-time venv (host Python is externally managed; `.docker/` is git-ignored):

```sh
just data-venv      # python3 -m venv .docker/pyenv + pip install -r tools/requirements.txt
just data-derive    # regenerate the map + catalogs from the PDFs
```

Re-run `data-derive` whenever Roland revises a PDF (new firmware), then review
the diff of the committed JSON like any code change. Both parsers **fail loud**
on anything they don't fully understand (unknown table rows, non-contiguous
numbering, off-grid addresses) and assert golden facts per device — for the V31
these are confirmed on real hardware (kit tempo at `00 6C`, 200 kits, …), for the
TD-17 they are read from the document only. A silent wrong catalog would be worse
than a crash.

## Parsing notes (why it's built this way)

- **MIDI Implementation** extracts fine with plain `pypdf`: section 3 is a
  monospaced ASCII table. The parser understands single-byte bit masks,
  `#`-marked multi-address nibble runs, per-character ASCII runs (1 byte or
  2 nibbles per char), container blocks, repeated entries (`Kit 1 … Kit 200` →
  count + stride in 7-bit address arithmetic), footnote ranges (`(*1)` — the row's
  meaning depends on a selection), **variant tables** that restate a block's
  offsets per pad model, per MFX type or per instrument group, and tables
  restated for **older firmware** (kept apart as `firmware_variants`).
- Adding the TD-17 cost five small generalizations and no module-specific code:
  tolerate indented table rows, accept a display row whose middle cell separator
  the PDF dropped, recognise the extra variant-table labels, keep a block's
  `Total Size` from being overwritten by a variant table's own, and lift `(*1)`
  footnotes out of parameter names. The V31 output was byte-identical afterwards
  apart from those cleaner names — which is the regression check to repeat when
  the next module lands: **re-derive every device and diff.**
- **Data List** tables need glyph positions (`pdfplumber`): the extracted text
  interleaves the two page columns and won't say where "kit name" ends and
  "sub name" begins. The PDF stores real space glyphs, so concatenating
  characters in x-order reproduces the exact authored strings; column
  boundaries come from each table's header row, and rows are clustered by
  baseline with a small tolerance (the `No.` column sits ~1pt off its row).
- Instrument names like `Ld VLAcrylic T1R` (no space) are **authentic** — the
  module's name field is 16 characters and Roland compresses to fit.

## Cassettes

`tools/cassettes/` holds recorded real-hardware sessions (NDJSON) used by the
`e2e` golden-replay tests — see the device-mock plan in
[TASKS.md](../TASKS.md); unrelated to the PDF parsers.
