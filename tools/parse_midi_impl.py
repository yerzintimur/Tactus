#!/usr/bin/env python3
"""Parse the Roland V31 MIDI Implementation PDF into a machine-readable address map.

Reads section "3. Parameter Address Map" of the vendor PDF (git-ignored under
docs/vendor/ — see ADR-0004) and emits our own derived JSON: the top-level
address table, every block table (container children or leaf parameters), and
the pad/bus assignment lists. The output is committed under profiles/maps/ and
cross-checked against profiles/roland-v31.json by Rust tests.

Usage:
    python3 tools/parse_midi_impl.py                      # default paths
    python3 tools/parse_midi_impl.py --pdf <pdf> --out <json>

Requires: pypdf (pip install pypdf).
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_PDF = REPO / "docs/vendor/V31_MIDI_Implementation_eng01_W.pdf"
DEFAULT_OUT = REPO / "profiles/maps/roland-v31-address-map.json"

SECTION_START = "3. Parameter Address Map"
SECTION_END = "4. Supplementary Material"

# ---------------------------------------------------------------- text layer

def extract_section(pdf_path: Path) -> list[str]:
    """All text lines between the section-3 heading and the section-4 heading."""
    from pypdf import PdfReader

    lines: list[str] = []
    for page in PdfReader(pdf_path).pages:
        lines.extend((page.extract_text() or "").splitlines())
    try:
        start = next(i for i, l in enumerate(lines) if l.strip() == SECTION_START)
        end = next(i for i, l in enumerate(lines) if l.strip() == SECTION_END)
    except StopIteration:
        sys.exit(f"error: could not find '{SECTION_START}' … '{SECTION_END}' in {pdf_path}")
    return lines[start + 1 : end]


# ------------------------------------------------------------- row grammars

HEX2 = r"[0-9A-F]{2}"
# | 04 00 00 00 | Kit 1                                                    [Kit] |
TOP_ROW = re.compile(
    rf"^\|\s*({HEX2}) ({HEX2}) ({HEX2}) ({HEX2})\s*\|\s*(.+?)\s*\[(\w+)\]\s*\|?\s*$"
)
# |    00 20 00 | Kit Unit Common 1                              [KitUnitCommon] |
CHILD_ROW = re.compile(
    rf"^\|\s*({HEX2}) ({HEX2}) ({HEX2})\s*\|\s*(.+?)\s*\[(\w+)\]\s*\|?\s*$"
)
# |#      00 6C | 0000 aaaa |                                                    |
# |       00 6F | 0000 dddd | Kit Tempo                             (200 - 2600) |
LEAF_ROW = re.compile(
    rf"^\|(#?)\s*({HEX2}) ({HEX2})\s*\|\s*([01a-d]{{4}} [01a-d]{{4}})\s*\|(.*?)\|?\s*$"
)
# |             |           |                                      20.0 - 260.0  |
DISPLAY_ROW = re.compile(r"^\|\s*\|\s*\|\s*(.+?)\s*\|?\s*$")
# | 00 00 00 04 |Total Size                                                      |
TOTAL_ROW = re.compile(rf"^\|\s*({HEX2}) ({HEX2}) ({HEX2}) ({HEX2})\s*\|\s*Total Size")
ELLIPSIS_ROW = re.compile(r"^\|\s*:\s*\|")
BLOCK_HEADING = re.compile(r"^\* \[(\w+)\]\s*$")
# Prose assignment lists:  "  [KitPad]"  /  "  [KitUnitCommon], [KitUnitLayer]"
ASSIGN_TARGETS = re.compile(r"^\s*((?:\[\w+\](?:,\s*)?)+)\s*$")
#   SNARE HEAD    2
ASSIGN_ENTRY = re.compile(r"^\s*([A-Z][A-Z0-9/() *\-]*?)\s+(\d+)\s*$")
# "Kit Unit LayerA 28" -> prefix "Kit Unit LayerA", index 28
TRAILING_INT = re.compile(r"^(.*\S)\s+(\d+)$")
# Per-pad-model variant tables inside [TrigDigital]: "Digital Pad: PD-14DSX"
OVERLAY_HEADING = re.compile(r"^Digital Pad:\s*(\S+)\s*$")
NAME_RANGE = re.compile(r"^(.*?)\s*\((-?\d+)\s*-\s*(-?\d*)\)\s*$")

SKIP = re.compile(
    r"^(\+[-+]+\+?|\|-+\+-+.*|\|\s*(Offset|Start)\s+\|.*|\|\s*Address\s*\|.*|\d+)$"
)


def addr_to_int(bytes_: list[int]) -> int:
    v = 0
    for b in bytes_:
        v = (v << 7) | b
    return v


def int_to_addr(v: int, width: int) -> list[int]:
    out = [0] * width
    for i in range(width - 1, -1, -1):
        out[i] = v & 0x7F
        v >>= 7
    if v:
        raise ValueError(f"address overflow for width {width}")
    return out


@dataclass
class Param:
    name: str
    offset: list[int]
    len: int
    encoding: str  # plain7 | nibble | ascii
    bits: int | None = None  # mask width for plain7 singles
    bytes_per_char: int | None = None  # ascii only: 2 => nibble-packed characters
    range: list[int] | None = None
    display: list[str] = field(default_factory=list)


@dataclass
class Entry:  # one top-level or container-child entry (possibly repeated)
    name: str
    block: str
    address: list[int]
    count: int | None = None
    stride: list[int] | None = None


# ------------------------------------------------------------------ parsing

def parse(lines: list[str]) -> dict:
    top: list[Entry] = []
    blocks: dict[str, dict] = {}
    assignments: dict[str, dict[str, str]] = {}

    current_block: str | None = None  # None => the top-level table
    params: list[Param] = []
    children_raw: list[tuple[list[int], str, str]] = []
    total_size: list[int] | None = None
    pending_run: list[tuple[list[int], str]] | None = None  # nibble run in flight
    assign_targets: list[str] = []
    current_overlay: str | None = None  # per-pad-model variant table in flight
    overlays: dict[str, list[Param]] = {}
    unparsed: list[str] = []

    def target() -> list[Param]:
        return overlays[current_overlay] if current_overlay else params

    def close_run_singleton():
        nonlocal pending_run
        if pending_run:
            raise SystemExit(
                f"error: nibble run never closed in block {current_block}: {pending_run}"
            )

    def flush_block():
        nonlocal params, children_raw, total_size, current_block, current_overlay, overlays
        close_run_singleton()
        if current_block is None:
            pass
        elif params:
            if children_raw:
                raise SystemExit(f"error: block {current_block} mixes params and children")
            blocks[current_block] = {
                "kind": "leaf",
                "params": [vars(p) for p in collapse_ascii(params)],
                **({"total_size": total_size} if total_size else {}),
                **(
                    {
                        "overlays": {
                            k: [vars(p) for p in collapse_ascii(v)] for k, v in overlays.items()
                        }
                    }
                    if overlays
                    else {}
                ),
            }
        else:
            blocks[current_block] = {
                "kind": "container",
                "children": [vars(e) for e in group_entries(children_raw)],
            }
        params, children_raw, total_size = [], [], None
        current_overlay, overlays = None, {}

    for raw in lines:
        line = raw.rstrip()
        if not line.strip() or SKIP.match(line.strip()):
            continue

        m = BLOCK_HEADING.match(line)
        if m:
            flush_block()
            current_block = m.group(1)
            assign_targets = []
            continue

        m = OVERLAY_HEADING.match(line)
        if m and current_block is not None:
            close_run_singleton()
            current_overlay = m.group(1)
            overlays[current_overlay] = []
            continue

        if ELLIPSIS_ROW.match(line):
            continue  # grouping is recomputed from first/last rows

        m = TOTAL_ROW.match(line)
        if m:
            total_size = [int(g, 16) for g in m.groups()]
            continue

        m = LEAF_ROW.match(line)
        if m:
            hash_, hi, lo, pattern, rest = m.groups()
            offset = [int(hi, 16), int(lo, 16)]
            rest = rest.strip()
            if hash_ or (pending_run and not pattern.startswith("0000 aaaa")):
                # part of a multi-address nibble run
                if hash_:
                    close_run_singleton()
                    pending_run = []
                if pending_run is None:
                    raise SystemExit(f"error: run continuation without start: {line}")
                pending_run.append((offset, rest))
                if rest:  # the closing line carries the name (+ range)
                    start_off = pending_run[0][0]
                    target().append(make_param(rest, start_off, len(pending_run), None))
                    pending_run = None
                continue
            close_run_singleton()
            bits = sum(c not in "01 " for c in pattern)
            target().append(make_param(rest, offset, 1, bits))
            continue

        m = DISPLAY_ROW.match(line)
        if m and current_block and target():
            target()[-1].display.append(m.group(1))
            continue

        m = TOP_ROW.match(line)
        if m and current_block is None:
            g = m.groups()
            top.append(Entry(g[4], g[5], [int(x, 16) for x in g[:4]]))
            continue

        m = CHILD_ROW.match(line)
        if m and current_block is not None:
            hi, mid, lo, name, block = m.groups()
            children_raw.append(([int(hi, 16), int(mid, 16), int(lo, 16)], name, block))
            continue

        m = ASSIGN_TARGETS.match(line)
        if m and current_block is not None and not params and not children_raw:
            assign_targets = re.findall(r"\[(\w+)\]", m.group(1))
            continue

        m = ASSIGN_ENTRY.match(line)
        if m and assign_targets:
            name, idx = m.group(1).strip(), m.group(2)
            for tgt in assign_targets:
                assignments.setdefault(tgt, {})[idx] = name
            continue

        if line.startswith("|"):
            unparsed.append(line)

    flush_block()

    if unparsed:
        for l in unparsed:
            print(f"warning: unparsed table line: {l!r}", file=sys.stderr)

    return {
        "top_level": [vars(e) for e in group_entries([(e.address, e.name, e.block) for e in top])],
        "blocks": blocks,
        "assignments": assignments,
    }


def make_param(rest: str, offset: list[int], length: int, bits: int | None) -> Param:
    encoding = "nibble" if length > 1 else "plain7"
    m = NAME_RANGE.match(rest)
    if m and m.group(3) != "":
        name, lo, hi = m.group(1), int(m.group(2)), int(m.group(3))
        rng = [lo, hi]
    elif m:  # open range like "Instrument (0 - )"
        name, rng = m.group(1), [int(m.group(2)), None]
    else:
        name, rng = rest, None
    p = Param(name=name, offset=offset, len=length, encoding=encoding, range=rng)
    if length == 1:
        p.bits = bits
    return p


def collapse_ascii(params: list[Param]) -> list[Param]:
    """Fold 'Kit Name 1'…'Kit Name 16' per-character rows into one string param.

    Characters are stored either as one 7-bit byte each (Kit Name) or as a
    2-nibble pair each (Trigger Bank Name) — `bytes_per_char` records which.
    """
    out: list[Param] = []
    i = 0
    while i < len(params):
        p = params[i]
        m = TRAILING_INT.match(p.name)
        if m and m.group(2) == "1" and any("[ASCII]" in d for d in p.display):
            prefix = m.group(1)
            j = i
            run: list[Param] = []
            while j < len(params):
                mj = TRAILING_INT.match(params[j].name)
                if not (mj and mj.group(1) == prefix and int(mj.group(2)) == len(run) + 1):
                    break
                run.append(params[j])
                j += 1
            start, endp = run[0], run[-1]
            char_len = start.len
            span = addr_to_int(endp.offset) - addr_to_int(start.offset) + char_len
            if any(q.len != char_len for q in run) or span != len(run) * char_len:
                raise SystemExit(f"error: non-contiguous ASCII run for '{prefix}'")
            out.append(
                Param(
                    name=prefix,
                    offset=start.offset,
                    len=len(run) * char_len,
                    encoding="ascii",
                    bytes_per_char=char_len if char_len > 1 else None,
                    range=start.range,
                    display=[],
                )
            )
            i = j
        else:
            out.append(p)
            i += 1
    return out


def group_entries(rows: list[tuple[list[int], str, str]]) -> list[Entry]:
    """Fold 'Kit 1' / 'Kit 2' / … / 'Kit 200' rows into one entry with count+stride."""
    out: list[Entry] = []
    i = 0
    while i < len(rows):
        addr, name, block = rows[i]
        m = TRAILING_INT.match(name)
        # A run starts at index 1 and every following row shares prefix + block.
        if m and int(m.group(2)) == 1:
            prefix = m.group(1)
            run = [rows[i]]
            j = i + 1
            while j < len(rows):
                aj, nj, bj = rows[j]
                mj = TRAILING_INT.match(nj)
                if not (mj and mj.group(1) == prefix and bj == block):
                    break
                run.append(rows[j])
                j += 1
            if len(run) >= 2:
                first_addr, _, _ = run[0]
                last_addr, last_name, _ = run[-1]
                count = int(TRAILING_INT.match(last_name).group(2))
                width = len(first_addr)
                span = addr_to_int(last_addr) - addr_to_int(first_addr)
                if span % (count - 1) != 0:
                    raise SystemExit(f"error: non-uniform stride for '{prefix}'")
                stride = int_to_addr(span // (count - 1), width)
                # Every explicitly listed row must sit on the stride grid.
                for a, n, _ in run:
                    k = int(TRAILING_INT.match(n).group(2)) - 1
                    expect = addr_to_int(first_addr) + k * (span // (count - 1))
                    if addr_to_int(a) != expect:
                        raise SystemExit(f"error: off-grid entry '{n}' in '{prefix}'")
                out.append(Entry(prefix, block, first_addr, count, stride))
                i = j
                continue
        out.append(Entry(name, block, addr))
        i += 1
    return out


# --------------------------------------------------------------- validation

def validate(doc: dict) -> None:
    referenced = {e["block"] for e in doc["top_level"]}
    for b in doc["blocks"].values():
        if b["kind"] == "container":
            referenced |= {c["block"] for c in b["children"]}
    missing = referenced - doc["blocks"].keys()
    if missing:
        sys.exit(f"error: referenced blocks without a definition: {sorted(missing)}")

    def check_monotonic(name: str, params: list[dict]) -> int:
        prev_end = 0
        for p in params:
            off = addr_to_int(p["offset"])
            if off < prev_end:
                sys.exit(f"error: overlapping offsets in [{name}] at {p['name']}")
            prev_end = off + p["len"]
        return prev_end

    for name, b in doc["blocks"].items():
        if b["kind"] != "leaf":
            continue
        end = check_monotonic(name, b["params"])
        for pad, plist in b.get("overlays", {}).items():
            end = max(end, check_monotonic(f"{name}:{pad}", plist))
        if "total_size" in b and end > addr_to_int(b["total_size"]):
            sys.exit(f"error: params overrun total size in [{name}]")

    # Golden facts, validated live on the real V31 (docs/PROTOCOL.md).
    kit_common = {p["name"]: p for p in doc["blocks"]["KitCommon"]["params"]}
    tempo = kit_common["Kit Tempo"]
    assert tempo["offset"] == [0, 0x6C] and tempo["len"] == 4, tempo
    assert tempo["range"] == [200, 2600], tempo
    assert kit_common["Kit Name"]["len"] == 16
    kit = next(e for e in doc["top_level"] if e["block"] == "Kit")
    assert kit["address"] == [4, 0, 0, 0] and kit["count"] == 200
    assert kit["stride"] == [0, 4, 0, 0]
    current = doc["blocks"]["Current"]["params"][0]
    assert current["name"] == "KitNum" and current["range"] == [0, 199]


def compact(v):
    """Drop None values and empty display lists so the committed JSON stays tidy."""
    if isinstance(v, dict):
        return {k: compact(x) for k, x in v.items() if x is not None and x != []}
    if isinstance(v, list):
        return [compact(x) for x in v]
    return v


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--pdf", type=Path, default=DEFAULT_PDF)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = ap.parse_args()

    if not args.pdf.exists():
        sys.exit(
            f"error: {args.pdf} not found — download the Roland PDF into docs/vendor/ "
            "(see docs/vendor/README.md); it is deliberately not committed (ADR-0004)"
        )

    doc = {
        "schema_version": 1,
        "source": (
            "Derived from Roland 'V31 MIDI Implementation' (eng01), (c) Roland "
            "Corporation — facts (addresses/sizes/ranges) extracted by "
            "tools/parse_midi_impl.py. See docs/PROTOCOL.md."
        ),
        **parse(extract_section(args.pdf)),
    }
    validate(doc)
    doc = compact(doc)

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(doc, indent=1) + "\n")
    n_params = sum(
        len(b["params"]) for b in doc["blocks"].values() if b["kind"] == "leaf"
    )
    print(
        f"wrote {args.out} — {len(doc['top_level'])} top-level areas, "
        f"{len(doc['blocks'])} blocks, {n_params} parameters"
    )


if __name__ == "__main__":
    main()
