#!/usr/bin/env python3
"""Parse the Roland V31 Data List PDF into catalog JSON files.

Emits our own derived data (committed under profiles/catalogs/roland-v31/):
  drum-kits.json    — the 200 preset kit names + sub names
  instruments.json  — preset instruments + EXV expansion packs, with group and
                      remark flags (*M mic, *P positional, *X cross-stick,
                      *O overtone, *L lo-cut)
  fx-types.json     — the 95 Bus FX types (read from the committed address map,
                      which is the cheaper/steadier source for that enum)

The vendor PDF itself is git-ignored (docs/vendor/, ADR-0004).

Text is reconstructed from glyph positions (pdfplumber chars): the PDF stores
real space glyphs, so concatenating chars in stream order yields the exact
authored strings, and column boundaries come from each table's header row.

Usage:  .docker/pyenv/bin/python tools/parse_datalist.py   (see tools/README.md)
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_PDF = REPO / "docs/vendor/V31_DataList_eng02_W.pdf"
DEFAULT_MAP = REPO / "profiles/maps/roland-v31-address-map.json"
DEFAULT_OUT = REPO / "profiles/catalogs/roland-v31"

SOURCE = (
    "Derived from Roland 'V31 Data List' (eng02), (c) Roland Corporation — "
    "facts (names/numbers) extracted by tools/parse_datalist.py."
)

# The Data List's own instrument-group taxonomy (fail loud on anything else).
GROUPS = [
    "KICK OTHERS", "KICK ELEC", "KICK",
    "SNARE OTHERS", "SNARE ELEC", "SNARE",
    "CROSS STICK",
    "TOM OTHERS", "TOM ELEC", "TOM",
    "HI-HAT OTHERS", "HI-HAT FIXED", "HI-HAT ELEC", "HI-HAT",
    "CRASH", "RIDE", "CHINA", "SPLASH", "STACKED CYMBAL",
    "CYMBAL OTHERS", "CYMBAL ELEC", "CYMBAL",
    "PERCUSSION", "PERC ELEC", "PERC",
    "CLAP", "SOUND FX", "BLOCK/COWBELL", "BELL/CHIME/GONG",
    "ELEMENTS", "OFF",
]

REMARK_FLAGS = {
    "M": "supports Mic Size / Mic Distance editing (Mic Size: kick only)",
    "P": "positional sensing (strike position / rim depth changes the sound)",
    "X": "on a snare rim, pairs the matching cross-stick sound on XSTICK",
    "O": "supports Overtone editing",
    "L": "supports Lo Cut editing",
}

EXV_HEADING = re.compile(r"^(EXV\d+):\s*(.+)$")
FLAGS_ONLY = re.compile(r"^(\*[A-Z]\s*)+$")


# ------------------------------------------------------------- page scraping

def page_rows(page, header_words: list[str]) -> list[dict]:
    """Reconstruct table rows from glyph positions, in reading order.

    Returns dicts {"page", "column", "top", "cells": [str, …]} where cells are
    split at the x positions of `header_words` (one boundary per word; a word
    like "Instrument group" is matched by its first token). Rows are ordered
    (column, top): the V31 lists flow down the left column, then the right.
    """
    mid = page.width / 2

    # Bucket glyphs into (column, row). Cells of one visual row can sit at
    # slightly different baselines (the No. column is ~1pt off), so cluster
    # tops with a 2.5pt tolerance instead of naive rounding. The PDF stores
    # real space glyphs, so concatenating chars by x order reproduces the
    # exact authored strings.
    chars: dict[tuple[int, int], list] = {}
    for col_sel in (0, 1):
        col_chars = sorted(
            (c for c in page.chars if (c["x0"] >= mid) == bool(col_sel)),
            key=lambda c: c["top"],
        )
        cluster = None
        for c in col_chars:
            if cluster is None or c["top"] - cluster > 2.5:
                cluster = c["top"]
            chars.setdefault((col_sel, round(cluster)), []).append(c)
    for cs in chars.values():
        cs.sort(key=lambda c: c["x0"])

    # Column boundaries from each column's header row, located glyph-wise (the
    # extractor's word splitting is kerning-mangled, chars are not).
    header_norm = "".join(h.replace(" ", "") for h in header_words)
    starts = [0]
    for h in header_words[:-1]:
        starts.append(starts[-1] + len(h.replace(" ", "")))
    bounds: dict[int, list[float]] = {}
    for (col, _top), cs in sorted(chars.items(), key=lambda kv: (kv[0][0], kv[0][1])):
        glyphs = [c for c in cs if c["text"].strip()]
        if "".join(c["text"] for c in glyphs) == header_norm:
            bounds.setdefault(col, [glyphs[i]["x0"] for i in starts])
    if not bounds:
        return []

    rows: list[dict] = []
    for (col, top), cs in chars.items():
        xs = bounds.get(col) or bounds[min(bounds)]
        cells = [""] * len(xs)
        for c in cs:
            field = 0
            for i, x in enumerate(xs):
                if c["x0"] >= x - 1.5:
                    field = i
            cells[field] += c["text"]
        rows.append({"column": col, "top": top, "cells": [c.strip() for c in cells]})
    rows.sort(key=lambda r: (r["column"], r["top"]))
    return rows


def find_pages(pdf, title: str) -> list[int]:
    """0-based indexes of pages whose headline contains `title`.

    Extracted headlines carry kerning artifacts ("Drum k it list"), so compare
    with all whitespace removed.
    """
    want = title.lower().replace(" ", "")
    out = []
    for i, page in enumerate(pdf.pages):
        head = (page.extract_text() or "").splitlines()[0:6]
        if any(want in l.lower().replace(" ", "") for l in head):
            out.append(i)
    if not out:
        sys.exit(f"error: no pages titled {title!r} found")
    return out


# ------------------------------------------------------------------ kit list

def parse_kits(pdf) -> list[dict]:
    kits = []
    for idx in find_pages(pdf, "Drum kit list"):
        for row in page_rows(pdf.pages[idx], ["No.", "Drum kit name", "Sub name"]):
            no, name, sub = row["cells"]
            if not no.isdigit() or int(no) > 998 or not name:  # skip page footers
                continue
            kits.append({"number": int(no), "name": name, "sub_name": sub})
    kits.sort(key=lambda k: k["number"])
    if [k["number"] for k in kits] != list(range(1, len(kits) + 1)):
        sys.exit("error: drum kit numbering is not contiguous from 1")
    return kits


# --------------------------------------------------------------- instruments

def split_group(cell: str) -> str:
    for g in GROUPS:  # longest-first ordering above
        if cell == g or cell.startswith(g + " "):
            return g
    sys.exit(f"error: unknown instrument group in {cell!r}")


def parse_instruments(pdf) -> tuple[list[dict], list[dict]]:
    preset: list[dict] = []
    expansions: list[dict] = []
    current: list[dict] | None = None  # None => preset section

    for idx in find_pages(pdf, "Instrument list"):
        page = pdf.pages[idx]
        rows = page_rows(
            page, ["No.", "Instrument group", "Instrument name", "Remarks"]
        )
        for row in rows:
            no, group_cell, name, remarks = row["cells"]
            # Headings span cell boundaries; the field cuts land inside words,
            # so re-joining without separators restores the authored text.
            m = EXV_HEADING.match("".join(row["cells"]))
            if m:
                expansions.append(
                    {"id": m.group(1), "title": m.group(2).strip(), "instruments": []}
                )
                current = expansions[-1]["instruments"]
                continue
            if not no.isdigit() or int(no) > 998 or not group_cell:  # page footers
                continue
            if remarks and not FLAGS_ONLY.match(remarks):
                sys.exit(f"error: unexpected remarks {remarks!r} for {name!r}")
            entry = {
                "number": int(no),
                "group": split_group(group_cell),
                "name": name,
                "flags": re.findall(r"\*([A-Z])", remarks),
            }
            (preset if current is None else current).append(entry)

    for flag in {f for i in preset for f in i["flags"]}:
        if flag not in REMARK_FLAGS:
            sys.exit(f"error: unknown remark flag *{flag}")
    preset.sort(key=lambda i: i["number"])
    if [i["number"] for i in preset] != list(range(len(preset))):
        sys.exit("error: preset instrument numbering is not contiguous from 0")
    for exp in expansions:
        nums = [i["number"] for i in exp["instruments"]]
        if nums != list(range(1, len(nums) + 1)):
            sys.exit(f"error: {exp['id']} numbering is not contiguous from 1")
    return preset, expansions


# ------------------------------------------------------------------ FX types

def fx_types_from_map(map_path: Path) -> list[str]:
    doc = json.loads(map_path.read_text())
    fx = next(p for p in doc["blocks"]["KitFx"]["params"] if p["name"] == "Bus Fx Type")
    names = [n.strip() for n in " ".join(fx["display"]).split(",")]
    lo, hi = fx["range"]
    if len(names) != hi - lo + 1:
        sys.exit(f"error: Bus Fx Type has {len(names)} labels for range {fx['range']}")
    return names


# ------------------------------------------------------------------- output

def write(path: Path, doc: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(doc, indent=1, ensure_ascii=False) + "\n")
    print(f"wrote {path}")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--pdf", type=Path, default=DEFAULT_PDF)
    ap.add_argument("--map", type=Path, default=DEFAULT_MAP)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = ap.parse_args()

    if not args.pdf.exists():
        sys.exit(
            f"error: {args.pdf} not found — download the Roland PDF into docs/vendor/ "
            "(see docs/vendor/README.md); it is deliberately not committed (ADR-0004)"
        )

    import pdfplumber

    with pdfplumber.open(args.pdf) as pdf:
        kits = parse_kits(pdf)
        preset, expansions = parse_instruments(pdf)

    write(
        args.out / "drum-kits.json",
        {"schema_version": 1, "source": SOURCE, "kits": kits},
    )
    write(
        args.out / "instruments.json",
        {
            "schema_version": 1,
            "source": SOURCE,
            "remark_flags": REMARK_FLAGS,
            "preset": preset,
            "expansions": expansions,
        },
    )
    write(
        args.out / "fx-types.json",
        {
            "schema_version": 1,
            "source": SOURCE + " FX type list read from the parsed address map.",
            "types": fx_types_from_map(args.map),
        },
    )
    n_exp = sum(len(e["instruments"]) for e in expansions)
    print(
        f"catalogs: {len(kits)} kits, {len(preset)} preset instruments, "
        f"{len(expansions)} expansion packs ({n_exp} instruments)"
    )


if __name__ == "__main__":
    main()
