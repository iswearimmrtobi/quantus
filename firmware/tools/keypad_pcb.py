#!/usr/bin/env python3
"""Rebuild the key matrix of the one-board keypad project on the physical grid and route the board.

See firmware/docs/keypad-pcb.md for what this does and why. This is still the one-board router
(13 rows on U1, columns on U2, GPB0 of U1 the unit strap); the split into a top and a bottom board
(keypad_split.py) needs a `--part top|bottom` mode with one 8 x 8 matrix per board, not built yet.

    python3 -I firmware/tools/keypad_pcb.py PROJECT                   # table + edit + route + check + write
    python3 -I firmware/tools/keypad_pcb.py PROJECT --table           # only print the new matrix
    python3 -I firmware/tools/keypad_pcb.py PROJECT --render out.png  # also draw both layers

PROJECT is the EasyEDA Pro project, left untouched; the output goes next to it, its name ending in
`_rerouted` (or `--out`). The project is a zip with one `*.epru` file of JSON lines
`{header}||{body}|`. Records that are not changed keep their original text, so a diff of the two
`.epru` files shows only the edits listed in the docs.

Units are mil (the file's unit), x to the right, y up; the board is x 0..2874, y -4527..0.
"""
import argparse
import heapq
import json
import math
import os
import random
import re
import sys
import zipfile
from collections import defaultdict


def beside(path, suffix):
    """`path` with `suffix` added to its name, before the extension."""
    stem, ext = os.path.splitext(path)
    return stem + suffix + ext


# ---------------------------------------------------------------------------------------------
# The keypad as designed (design/keypad-sizes.html): the legend of every key, block by block.
# '.' is no key.

SMALL = {
    "soft": ["F1 F2 F3 F4 F5", "F6 F7 F8 F9 F10"],
    "control": ["SHIFT ALPHA UNIT OPTN MODE ON"],
    "function": [
        "SYMB a/b √ . ▲ .",
        "VARS x² xʸ ◄ OK ►",
        "CALC log ln . ▼ .",
        "(−) x⁻¹ sin cos tan π",
        "( ) = ; CONST ENG",
        "x y e i CONV S⇔D",
    ],
    "number": ["7 8 9 DEL AC", "4 5 6 × ÷", "1 2 3 + −", "0 , ×10ˣ Ans EXE"],
}
BIG = {
    "soft": ["F1 F2 F3 F4 F5 F6", "F7 F8 F9 F10 F11 F12"],
    "control": ["SHIFT ALPHA UNIT OPTN . MODE ON"],
    "function": [
        "SYMB a/b √ x² . ▲ .",
        "VARS xʸ log ln ◄ OK ►",
        "CALC UNDO ▶unit x³ . ▼ .",
        "(−) x⁻¹ sin cos tan π 10ˣ",
        "( ) = ; CONST ENG Σ",
        "x y e i CONV S⇔D ∫",
    ],
    "number": ["7 8 9 DEL AC", "4 5 6 × ÷", "1 2 3 + −", "0 , ×10ˣ Ans EXE"],
}


def grid_of(layout):
    """{legend: (row, col)}: the physical grid. Rows count from the top over all blocks (soft,
    control, function + arrows, number), the column is the key's place in its block's row."""
    cells, r = {}, 0
    for block in ("soft", "control", "function", "number"):
        for line in layout[block]:
            for c, tok in enumerate(line.split()):
                if tok != "." and tok != "ON":
                    assert tok not in cells, tok
                    cells[tok] = (r, c)
            r += 1
    return cells


ROWS = 13  # 2 soft + 1 control + 6 function/arrow + 4 number
COLS_SMALL, COLS_BIG = 6, 7

# Which MCP23017 pin carries which row (any GPIO does the same job; GPB0 of U1 is the unit strap).
# Chip pins: GPA0-7 = 21..28, GPB0-7 = 1..8.
ROW_PIN = {}
COL_PIN = {}


def gpa(n):
    return 21 + n


def gpb(n):
    return 1 + n


def set_pins(row_pins=None):
    ROW_PIN.clear()
    COL_PIN.clear()
    order = row_pins or ([gpa(i) for i in range(8)] + [gpb(i) for i in range(1, 6)])
    for r, p in enumerate(order):
        ROW_PIN[r] = p
    for c in range(COLS_SMALL):
        COL_PIN[c] = gpa(c)


set_pins()

# ---------------------------------------------------------------------------------------------
# Reading and writing the project


class Rec:
    __slots__ = ("head", "body", "raw", "dirty", "doc")

    def __init__(self, head, body, raw, doc):
        self.head, self.body, self.raw, self.dirty, self.doc = head, body, raw, False, doc

    @property
    def type(self):
        return self.head["type"]

    @property
    def id(self):
        return self.head["id"]

    def text(self):
        if not self.dirty:
            return self.raw
        return (json.dumps(self.head, separators=(",", ":"), ensure_ascii=False) + "||"
                + json.dumps(self.body, separators=(",", ":"), ensure_ascii=False) + "|")


class Doc:
    def __init__(self, kind, uuid, head_rec):
        self.kind, self.uuid, self.head_rec, self.recs = kind, uuid, head_rec, []


class Project:
    def __init__(self, path):
        self.path = path
        with zipfile.ZipFile(path) as z:
            self.names = z.namelist()
            self.files = {n: z.read(n) for n in self.names}
            self.infos = {i.filename: i for i in z.infolist()}
        self.epru_name = next(n for n in self.names if n.endswith(".epru"))
        raw = self.files[self.epru_name].decode("utf-8")
        self.docs, self.all = [], []
        cur = None
        for line in raw.split("\n"):
            if not line:
                continue
            a, sep, b = line.partition("||")
            head = json.loads(a)
            if head["type"] == "DOCHEAD":
                body = json.loads(b[:-1])
                cur = Doc(body["docType"], body["uuid"], Rec(head, body, line, None))
                cur.head_rec.doc = cur
                self.docs.append(cur)
                self.all.append(cur.head_rec)
                continue
            body = json.loads(b[:-1]) if b.endswith("|") and b[:-1] else (json.loads(b) if b else None)
            rec = Rec(head, body, line, cur)
            cur.recs.append(rec)
            self.all.append(rec)

    def doc(self, kind, n=0):
        return [d for d in self.docs if d.kind == kind][n]

    def write(self, path):
        lines = []
        for d in self.docs:
            lines.append(d.head_rec.text())
            lines.extend(r.text() for r in d.recs if r.body is not False)
        data = ("\n".join(lines) + "\n") if self.files[self.epru_name].endswith(b"\n") else "\n".join(lines)
        out = dict(self.files)
        out[self.epru_name] = data.encode("utf-8")
        with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
            for n in self.names:
                z.writestr(n, out[n])


# ---------------------------------------------------------------------------------------------
# Geometry of the board


class Pad:
    def __init__(self, comp, num, x, y, w, h, shape, layer):
        self.comp, self.num, self.x, self.y, self.w, self.h = comp, num, x, y, w, h
        self.shape, self.layer, self.net = shape, layer, ""

    @property
    def key(self):
        return (self.comp, self.num)


class Board:
    def __init__(self, prj):
        self.prj = prj
        pcb = prj.doc("PCB")
        self.pcb = pcb
        foot = {}
        for d in prj.docs:
            if d.kind == "FOOTPRINT":
                meta = next(r.body for r in d.recs if r.type == "META")
                foot[d.uuid] = (meta["title"], [r.body for r in d.recs if r.type == "PAD"])
        self.comp = {}
        attrs = defaultdict(dict)
        for r in pcb.recs:
            if r.type == "ATTR":
                attrs[r.body["parentId"]][r.body["key"]] = r
        self.pads = {}
        self.parts = {}
        for r in pcb.recs:
            if r.type != "COMPONENT":
                continue
            a = attrs[r.id]
            des = a["Designator"].body["value"]
            title, fpads = foot[a["Footprint"].body["value"]]
            c = r.body
            bottom = c["layerId"] == 2
            ang = math.radians(c["angle"])
            self.parts[des] = dict(id=r.id, x=c["x"], y=c["y"], angle=c["angle"], bottom=bottom, rec=r, foot=title)
            for p in fpads:
                px, py = p["centerX"], p["centerY"]
                if bottom:
                    px = -px
                qx = px * math.cos(ang) - py * math.sin(ang)
                qy = px * math.sin(ang) + py * math.cos(ang)
                w, h = p["defaultPad"]["width"], p["defaultPad"]["height"]
                if round(c["angle"] / 90) % 2:
                    w, h = h, w
                pad = Pad(des, p["num"], c["x"] + qx, c["y"] + qy, w, h, p["defaultPad"]["padType"],
                          2 if bottom else 1)
                self.pads[pad.key] = pad
        self.pad_net_recs = {}
        for r in pcb.recs:
            if r.type == "PAD_NET":
                _, cid, num, _ = json.loads(r.id)
                des = next(k for k, v in self.parts.items() if v["id"] == cid)
                self.pad_net_recs[(des, num)] = r
                self.pads[(des, num)].net = r.body["padNet"]
        outline = next(r.body for r in pcb.recs if r.type == "POLY" and r.body.get("polyType") == "BOARD_OUTLINE")
        _, x, y, w, h = outline["path"][:5]
        self.outline = (x, y - h, x + w, y)  # xmin, ymin, xmax, ymax
        assert self.outline[3] == 0 or True

    def nets(self):
        d = defaultdict(list)
        for p in self.pads.values():
            if p.net:
                d[p.net].append(p)
        return d


# ---------------------------------------------------------------------------------------------
# The new matrix


def switch_cells(board):
    """{SWn: (legend, row, col)} from the PCB positions and the small layout's legends."""
    sws = {d: p for d, p in board.parts.items() if d.startswith("SW")}
    # physical rows of the board: sort by y (top first), cluster
    ys = sorted({round(p["y"]) for p in sws.values()}, reverse=True)
    rows = []
    for y in ys:
        if rows and abs(rows[-1][0] - y) < 40:
            continue
        rows.append([y])
    rowy = [r[0] for r in rows]

    def row_of(y):
        return min(range(len(rowy)), key=lambda i: abs(rowy[i] - y))

    by_row = defaultdict(list)
    for d, p in sws.items():
        by_row[row_of(p["y"])].append((p["x"], d))
    # legend rows top to bottom, ON excluded (it has its own place, the power latch)
    legend_rows = []
    for block in ("soft", "control", "function", "number"):
        for line in SMALL[block]:
            legend_rows.append([t for t in line.split() if t != "."])
    # the control row holds ON too: the 6th key on the PCB
    assert len(rowy) == len(legend_rows) == ROWS, (len(rowy), len(legend_rows))
    out = {}
    small = grid_of(SMALL)
    for i, toks in enumerate(legend_rows):
        keys = sorted(by_row[i])
        assert len(keys) == len(toks), (i, toks, keys)
        for (x, d), tok in zip(keys, toks):
            if tok == "ON":
                out[d] = ("ON", None, None)
            else:
                out[d] = (tok, ) + small[tok]
    return out


def net_row(r):
    return f"ROW{r}"


def net_col(c):
    return f"COL{c}"


# ---------------------------------------------------------------------------------------------
# Editing the schematic and the PCB nets


class Ids:
    """Fresh tickets and ids for new records."""

    def __init__(self, prj):
        self.ticket = max(r.head.get("ticket", 0) for r in prj.all if r.type != "DOCHEAD") + 10
        self.rng = random.Random(20261109)
        self.used = {r.head["id"] for r in prj.all if r.type != "DOCHEAD"}

    def tick(self):
        self.ticket += 1
        return self.ticket

    def hexid(self):
        while True:
            i = "%016x" % self.rng.getrandbits(64)
            if i not in self.used:
                self.used.add(i)
                return i


def sch_pages(prj):
    return [d for d in prj.docs if d.kind == "SCH_PAGE"]


def sch_page_title(d):
    return next(r.body for r in d.recs if r.type == "META")["title"]


def edit_schematic(prj, cells, ids, log):
    """Netports of the switches, their labels, and the chip pins of the matrix_input page."""
    for d in sch_pages(prj):
        title = sch_page_title(d)
        comps = {r.id: r for r in d.recs if r.type == "COMPONENT"}
        attrs = defaultdict(dict)
        for r in d.recs:
            if r.type == "ATTR":
                attrs[r.body["parentId"]][r.body["key"]] = r
        if title.endswith("_btn"):
            at = {}  # (x, y) -> netport component
            for cid, c in comps.items():
                if c.body["partId"] == "pid8a0e77bacb214e" and "Name" in attrs[cid] and "Designator" not in attrs[cid]:
                    at[(c.body["x"], c.body["y"])] = cid
            texts = {(r.body["x"], r.body["y"]): r for r in d.recs if r.type == "TEXT"}
            for cid, c in comps.items():
                a = attrs[cid]
                if "Designator" not in a or a["Designator"].body["value"] not in cells:
                    continue
                des = a["Designator"].body["value"]
                legend, row, col = cells[des]
                if row is None:
                    continue
                x, y = c.body["x"], c.body["y"]
                for dx, net in ((-40, net_row(row)), (40, net_col(col))):
                    rec = attrs[at[(x + dx, y)]]["Name"]
                    log.append((title, des, rec.body["value"], net))
                    rec.body["value"] = net
                    rec.dirty = True
                t = texts[(x - 30, y - 28)]
                t.body["value"] = f"{legend}  R{row}C{col}"
                t.dirty = True
            for t in d.recs:
                if t.type == "TEXT" and "hardware.md" in t.body["value"]:
                    t.body["value"] = t.body["value"].replace("hardware.md §3", "keypad-pcb.md")
                    t.dirty = True
        elif title == "matrix_input":
            edit_matrix_page(d, comps, attrs, ids)


def edit_matrix_page(d, comps, attrs, ids):
    NETPORT = "pid8a0e77bacb214e"
    chips = {attrs[c]["Designator"].body["value"]: c for c in comps if "Designator" in attrs[c]
             and attrs[c]["Designator"].body["value"] in ("U1", "U2")}
    pat = re.compile(r"^(ROW|COL)\d+$")
    # template records: a netport on the right (U1 ROW0) and on the left (U2 COL8)
    def pull(cid):
        c = comps[cid]
        a = list(attrs[cid].values())
        wire_line = None
        for r in d.recs:
            if r.type == "LINE" and isinstance(r.body, dict) and abs(r.body["startX"] - c.body["x"]) <= 10 and r.body["startY"] == c.body["y"] \
                    and abs(r.body["endX"] - c.body["x"]) <= 10 and r.body["endY"] == c.body["y"]:
                wire_line = r
        return c, a, wire_line

    victims = [cid for cid, c in comps.items() if c.body["partId"] == NETPORT and "Name" in attrs[cid]
               and "Designator" not in attrs[cid] and pat.match(attrs[cid]["Name"].body["value"])]
    right_t = next(cid for cid in victims if attrs[cid]["Name"].body["value"] == "ROW0")
    left_t = next(cid for cid in victims if attrs[cid]["Name"].body["value"] == "COL8")
    tmpl = {"R": pull(right_t), "L": pull(left_t)}
    tmpl = {k: (copy_body(c.body), [copy_body(a.body) for a in al], copy_body(wl.body)) for k, (c, al, wl) in tmpl.items()}
    gone = set()
    for cid in victims:
        c, al, wl = pull(cid)
        gone.add(cid)
        c.body = False
        for a in al:
            a.body = False
        wire_id = wl.body["lineGroup"]
        wl.body = False
        for r in d.recs:
            if r.type == "WIRE" and r.id == wire_id:
                r.body = False
    zmax = max((r.body.get("zIndex") or 0) for r in d.recs if isinstance(r.body, dict))
    new_recs = []

    def place(chip, pin, net):
        c = comps[chips[chip]].body
        if pin <= 14:
            side, px, py = "L", c["x"] - 50, c["y"] - 65 + 10 * (pin - 1)
        else:
            side, px, py = "R", c["x"] + 50, c["y"] - 65 - 10 * (pin - 28)
        cbody, abodies, wbody = tmpl[side]
        nx = px - 10 if side == "L" else px + 10
        cid, wid = ids.hexid(), ids.hexid()
        nonlocal zmax
        zmax += 1
        wire = Rec({"type": "WIRE", "ticket": ids.tick(), "id": wid}, {"zIndex": zmax}, "", d)
        wire.dirty = True
        zmax += 1
        line = Rec({"type": "LINE", "ticket": ids.tick(), "id": ids.hexid()},
                   dict(wbody, startX=px, startY=py, endX=nx, endY=py, lineGroup=wid), "", d)
        line.dirty = True
        zmax += 1
        comp = Rec({"type": "COMPONENT", "ticket": ids.tick(), "id": cid},
                   dict(cbody, x=nx, y=py, zIndex=zmax), "", d)
        comp.dirty = True
        out = [wire, line, comp]
        dx = nx - cbody_orig_x[side]
        for ab in abodies:
            b = dict(ab, parentId=cid)
            if ab["key"] == "Name":
                b["value"] = net
                b["x"] = nx + (45 if side == "R" else -45)
                b["y"] = py
            elif ab["key"] == "Symbol":
                b["x"], b["y"] = nx, py - 30
            rr = Rec({"type": "ATTR", "ticket": ids.tick(), "id": ids.hexid()}, b, "", d)
            rr.dirty = True
            out.append(rr)
        new_recs.extend(out)

    cbody_orig_x = {"R": tmpl["R"][0]["x"], "L": tmpl["L"][0]["x"]}
    for r, pin in ROW_PIN.items():
        place("U1", pin, net_row(r))
    for c, pin in COL_PIN.items():
        place("U2", pin, net_col(c))
    d.recs.extend(new_recs)


def copy_body(b):
    return json.loads(json.dumps(b))


def edit_pcb_nets(prj, board, cells, ids, log):
    """Pad nets of switches and chips, the net list and the silk labels; the old tracks go."""
    pcb = board.pcb
    for des, (legend, row, col) in cells.items():
        if row is None:
            continue
        for num, net in (("1", net_row(row)), ("2", net_col(col))):
            rec = board.pad_net_recs[(des, num)]
            log.append(("PCB", des, rec.body["padNet"], net))
            rec.body["padNet"] = net
            rec.dirty = True
            board.pads[(des, num)].net = net
    for chip, pins in (("U1", ROW_PIN), ("U2", COL_PIN)):
        want = {str(p): (net_row(r) if chip == "U1" else net_col(r)) for r, p in pins.items()}
        for (des, num), rec in board.pad_net_recs.items():
            if des != chip:
                continue
            old = rec.body["padNet"]
            new = want.get(num, "" if re.match(r"^(ROW|COL)\d+$", old) else old)
            if new != old:
                log.append(("PCB", f"{des}.{num}", old, new))
                rec.body["padNet"] = new
                rec.dirty = True
                board.pads[(des, num)].net = new
    # net list
    used = {p.net for p in board.pads.values() if p.net}
    netrecs = {json.loads(r.id)[1]: r for r in pcb.recs if r.type == "NET"}
    for name, r in list(netrecs.items()):
        if re.match(r"^(ROW|COL)\d+$", name) and name not in used:
            r.body = False
    tmpl = netrecs["ROW0"]
    pos = pcb.recs.index(tmpl)
    for name in sorted(used - set(netrecs), key=lambda s: (s[:3], int(s[3:]))):
        rec = Rec({"type": "NET", "ticket": ids.tick(), "id": json.dumps(["NET", name], separators=(",", ":"))},
                  copy_body(tmpl.body), "", pcb)
        rec.dirty = True
        pcb.recs.insert(pos + 1, rec)
    # silk labels "SWn RxCy"
    for r in pcb.recs:
        if r.type == "STRING":
            m = re.match(r"^(SW\d+) R\d+C\d+$", r.body["text"])
            if m and cells[m.group(1)][1] is not None:
                r.body["text"] = f"{m.group(1)} R{cells[m.group(1)][1]}C{cells[m.group(1)][2]}"
                r.dirty = True
    # old copper
    for r in pcb.recs:
        if r.type in ("LINE", "VIA") and re.match(r"^(ROW|COL)\d+$", r.body.get("netName", "")):
            r.body = False


# ---------------------------------------------------------------------------------------------
# The router: two layers, a 5 mil grid, 8 directions, A* from the routed part of a net to its next
# pad. Rules are those of the project (JLCPCB capability): 10 mil tracks, 6 mil clearance (the
# router keeps 10, the grid costs up to 3.5 mil), vias 12/24 mil.

import numpy as np

G = 5.0                 # grid step
TW = 10.0               # track width
CLR = 10.0              # clearance kept while routing
DRC_CLR = 6.0           # clearance the rules ask for (checked afterwards)
VIA_D, VIA_H = 24.0158, 12.0078
VIA_R = VIA_D / 2
EDGE = 20.0             # track centre to the board outline
BLOCK = 255
VIA_COST = 40.0
BEND45, BEND90 = 3.0, 10.0
WRONG_WAY = 1.35        # cost factor against the preferred direction of a layer
DIRS = [(1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1), (0, -1), (1, -1)]
TOP, BOT = 0, 1


class Grid:
    def __init__(self, board):
        self.b = board
        x0, y0, x1, y1 = board.outline
        self.x0, self.y0 = x0, y0
        self.nx = int(round((x1 - x0) / G)) + 1
        self.ny = int(round((y1 - y0) / G)) + 1
        self.n = self.nx * self.ny
        self.t = [np.zeros(self.n, np.uint8) for _ in range(2)]   # track keepout owner per layer
        self.v = [np.zeros(self.n, np.uint8) for _ in range(2)]   # via keepout owner per layer
        self.mt = [memoryview(a) for a in self.t]
        self.mv = [memoryview(a) for a in self.v]
        self.netid = {}
        self.segs = []   # (net, layer, x1, y1, x2, y2)
        self.vias = []   # (net, x, y)
        self.edge()

    def cell(self, x, y):
        return int(round((x - self.x0) / G)), int(round((y - self.y0) / G))

    def xy(self, i, j):
        return self.x0 + i * G, self.y0 + j * G

    def edge(self):
        k = int(math.ceil(EDGE / G))
        for l in range(2):
            a = self.t[l].reshape(self.nx, self.ny)
            a[:k, :] = BLOCK; a[-k:, :] = BLOCK; a[:, :k] = BLOCK; a[:, -k:] = BLOCK
            a = self.v[l].reshape(self.nx, self.ny)
            kv = k + 2
            a[:kv, :] = BLOCK; a[-kv:, :] = BLOCK; a[:, :kv] = BLOCK; a[:, -kv:] = BLOCK

    def window(self, xmin, ymin, xmax, ymax):
        i0, j0 = self.cell(xmin, ymin)
        i1, j1 = self.cell(xmax, ymax)
        return max(i0 - 1, 0), max(j0 - 1, 0), min(i1 + 2, self.nx), min(j1 + 2, self.ny)

    def mark(self, arr, layer_dist, net, win):
        """Set the owner of the cells in `win` where mask is true; two owners make a cell unusable."""
        i0, j0, i1, j1 = win
        a = arr.reshape(self.nx, self.ny)[i0:i1, j0:j1]
        m = layer_dist
        free = m & (a == 0)
        a[free] = net
        clash = m & (a != 0) & (a != net)
        a[clash] = BLOCK

    def coords(self, win):
        i0, j0, i1, j1 = win
        xs = self.x0 + np.arange(i0, i1) * G
        ys = self.y0 + np.arange(j0, j1) * G
        return np.meshgrid(xs, ys, indexing="ij")

    def add_rect(self, net, layer, cx, cy, hw, hh, vias_too=True, other_layer_via=True):
        """Copper rectangle (a pad) of `net` on `layer`."""
        r = CLR + TW / 2
        rv = CLR + VIA_R
        win = self.window(cx - hw - rv, cy - hh - rv, cx + hw + rv, cy + hh + rv)
        X, Y = self.coords(win)
        dx = np.maximum(np.abs(X - cx) - hw, 0)
        dy = np.maximum(np.abs(Y - cy) - hh, 0)
        d = np.hypot(dx, dy)
        self.mark(self.t[layer], d <= r, net, win)
        self.mark(self.v[layer], d <= rv, net, win)
        # no vias in or right next to a pad, on either layer
        ban = d <= VIA_R + 2
        for l in range(2):
            a = self.v[l].reshape(self.nx, self.ny)[win[0]:win[2], win[1]:win[3]]
            a[ban] = BLOCK

    def add_seg(self, net, layer, x1, y1, x2, y2):
        self.segs.append((net, layer, x1, y1, x2, y2))
        r = CLR + TW / 2 + TW / 2
        rv = CLR + TW / 2 + VIA_R
        win = self.window(min(x1, x2) - rv, min(y1, y2) - rv, max(x1, x2) + rv, max(y1, y2) + rv)
        X, Y = self.coords(win)
        d = seg_dist(X, Y, x1, y1, x2, y2)
        self.mark(self.t[layer], d <= r, net, win)
        self.mark(self.v[layer], d <= rv, net, win)

    def add_via(self, net, x, y):
        self.vias.append((net, x, y))
        rt = VIA_R + CLR + TW / 2
        rv = VIA_R + CLR + VIA_R
        win = self.window(x - rv, y - rv, x + rv, y + rv)
        X, Y = self.coords(win)
        d = np.hypot(X - x, Y - y)
        for l in range(2):
            self.mark(self.t[l], d <= rt, net, win)
            self.mark(self.v[l], d <= rv, net, win)


def seg_dist(X, Y, x1, y1, x2, y2):
    dx, dy = x2 - x1, y2 - y1
    L = dx * dx + dy * dy
    if L == 0:
        return np.hypot(X - x1, Y - y1)
    t = np.clip(((X - x1) * dx + (Y - y1) * dy) / L, 0, 1)
    return np.hypot(X - (x1 + t * dx), Y - (y1 + t * dy))


class Router:
    def __init__(self, board, seed=0):
        self.board = board
        self.grid = Grid(board)
        g = self.grid
        nets = board.nets()
        self.names = sorted(nets)
        self.nid = {n: i + 1 for i, n in enumerate(self.names)}
        self.nets = nets
        self.access = {}
        self.paths = defaultdict(list)   # net -> list of (layer, [(x, y), ...]) polylines
        self.out_vias = defaultdict(list)
        for p in board.pads.values():
            if p.net:
                g.add_rect(self.nid[p.net], TOP if p.layer == 1 else BOT, p.x, p.y, p.w / 2, p.h / 2)
        for p in board.pads.values():
            if p.net:
                self.access[p.key] = self.pad_cells(p)
        # Fine-pitch pads get a straight escape stub first, so later nets cannot box them in.
        for p in board.pads.values():
            if p.net and p.comp in ("U1", "U2", "J1") and p.w < 30 and p.h > 50:
                self.stub(p)
        self.failed = []

    def stub(self, p, length=70.0):
        """Escape stub straight out of a pad, away from the part's centre line."""
        g = self.grid
        part = self.board.parts[p.comp]
        up = 1 if p.y > part["y"] else -1
        layer = TOP if p.layer == 1 else BOT
        i, j = g.cell(p.x, p.y + up * (p.h / 2 + length))
        x, y = g.xy(i, j)
        n = self.nid[p.net]
        pts = simplify([(p.x, p.y), (x, p.y), (x, y)])
        self.paths[p.net].append((layer, pts))
        for a, b in zip(pts, pts[1:]):
            g.add_seg(n, layer, a[0], a[1], b[0], b[1])
        self.access[p.key] = [(i, j)]

    def pad_cells(self, p):
        g = self.grid
        shrink = TW / 2 + 1
        hw, hh = max(p.w / 2 - shrink, 0), max(p.h / 2 - shrink, 0)
        i0, j0 = g.cell(p.x - hw, p.y - hh)
        i1, j1 = g.cell(p.x + hw, p.y + hh)
        cells = []
        for i in range(i0, i1 + 1):
            for j in range(j0, j1 + 1):
                x, y = g.xy(i, j)
                if abs(x - p.x) <= hw + 1e-6 and abs(y - p.y) <= hh + 1e-6:
                    cells.append((i, j))
        if not cells:
            cells = [g.cell(p.x, p.y)]
        return cells

    # --- search -------------------------------------------------------------------------------
    def route_net(self, name, order_hint=None):
        g = self.grid
        n = self.nid[name]
        pads = list(self.nets[name])
        if len(pads) < 2:
            return True
        ny, nx = g.ny, g.nx
        layer_of = lambda p: TOP if p.layer == 1 else BOT
        # tree: layer -> set of flat cell indices; start with the pad that is first (nearest to the others' centroid)
        cx = sum(p.x for p in pads) / len(pads)
        cy = sum(p.y for p in pads) / len(pads)
        pads.sort(key=lambda p: (p.x - cx) ** 2 + (p.y - cy) ** 2)
        first = pads.pop(0)
        tree = [set(), set()]
        self.add_pad_to_tree(tree, first, layer_of(first))
        todo = pads
        ok = True
        while todo:
            # nearest unrouted pad to the tree (manhattan to the pad centres of the tree's pads)
            tx = [(g.xy(*divmod(c, ny))) for l in range(2) for c in list(tree[l])[:: max(1, len(tree[l]) // 40)]]
            todo.sort(key=lambda p: min(abs(p.x - x) + abs(p.y - y) for x, y in tx))
            tgt = todo.pop(0)
            res = self.astar(n, tree, tgt, layer_of(tgt))
            if res is None:
                self.failed.append((name, tgt.key))
                ok = False
                continue
            self.commit(name, n, tree, res, tgt)
        return ok

    def add_pad_to_tree(self, tree, pad, layer):
        ny = self.grid.ny
        for (i, j) in self.access[pad.key]:
            tree[layer].add(i * ny + j)

    def astar(self, n, tree, tgt, tlayer):
        g = self.grid
        ny = g.ny
        mt, mv = g.mt, g.mv
        goal = {i * ny + j for (i, j) in self.access[tgt.key]}
        gx, gy = tgt.x, tgt.y
        gi, gj = g.cell(gx, gy)
        heap = []
        best = {}
        parent = {}
        cnt = 0
        for l in range(2):
            for c in tree[l]:
                st = (c * 2 + l) * 9 + 8
                best[st] = 0.0
                i, j = divmod(c, ny)
                h = octile(i - gi, j - gj)
                heap.append((h, cnt, st, 0.0))
                cnt += 1
        heapq.heapify(heap)
        offs = [di * ny + dj for di, dj in DIRS]
        costs = [G, G * 1.4142] * 4
        pref_h = (True, False)   # top prefers horizontal (x), bottom vertical (y)
        pops = 0
        while heap:
            pops += 1
            if pops > 3000000:
                return None
            f, _, st, gc = heapq.heappop(heap)
            if gc > best.get(st, 1e18) + 1e-9:
                continue
            d = st % 9
            lc = st // 9
            l = lc % 2
            c = lc // 2
            if c in goal and l == tlayer:
                # path back
                path = []
                s = st
                while True:
                    path.append(s)
                    s = parent.get(s)
                    if s is None:
                        break
                path.reverse()
                return [((p // 9) // 2, (p // 9) % 2) for p in path]
            i, j = divmod(c, ny)
            tl, vl = mt[l], mv[l]
            # moves in the layer
            for k in range(8):
                nc = c + offs[k]
                v = tl[nc]
                if v != 0 and v != n:
                    continue
                if k & 1:
                    # diagonal: both neighbours of the corner must be free too
                    di, dj = DIRS[k]
                    a = tl[c + di * ny]
                    b = tl[c + dj]
                    if (a != 0 and a != n) or (b != 0 and b != n):
                        continue
                    cost = costs[k]
                else:
                    cost = G
                dxy = DIRS[k]
                horizontal = dxy[1] == 0
                vertical = dxy[0] == 0
                if horizontal and not pref_h[l]:
                    cost *= WRONG_WAY
                elif vertical and pref_h[l]:
                    cost *= WRONG_WAY
                if d != 8 and d != k:
                    turn = min((k - d) % 8, (d - k) % 8)
                    cost += BEND45 if turn == 1 else BEND90
                ng = gc + cost
                ns = (nc * 2 + l) * 9 + k
                if ng < best.get(ns, 1e18):
                    best[ns] = ng
                    parent[ns] = st
                    ii, jj = divmod(nc, ny)
                    heapq.heappush(heap, (ng + octile(ii - gi, jj - gj), cnt, ns, ng))
                    cnt += 1
            # via
            o = 1 - l
            ov = g.mt[o][c]
            if (ov == 0 or ov == n) and tl[c] in (0, n):
                a, b = vl[c], mv[o][c]
                if (a == 0 or a == n) and (b == 0 or b == n):
                    ng = gc + VIA_COST
                    ns = (c * 2 + o) * 9 + 8
                    if ng < best.get(ns, 1e18):
                        best[ns] = ng
                        parent[ns] = st
                        heapq.heappush(heap, (ng + octile(i - gi, j - gj), cnt, ns, ng))
                        cnt += 1
        return None

    def commit(self, name, n, tree, res, tgt):
        """Turn a path of (cell, layer) into segments and vias, add it to the tree and the grid."""
        g = self.grid
        ny = g.ny
        pts = [(g.xy(*divmod(c, ny)), l) for c, l in res]
        # split at layer changes
        runs, cur = [], [pts[0]]
        for a, b in zip(pts, pts[1:]):
            if a[1] != b[1]:
                runs.append(cur)
                self.out_vias[name].append(a[0])
                g.add_via(n, *a[0])
                cur = [b]
            else:
                cur.append(b)
        runs.append(cur)
        for run in runs:
            layer = run[0][1]
            xy = [p for p, _ in run]
            xy = simplify(xy)
            if layer == (TOP if tgt.layer == 1 else BOT) and run is runs[-1]:
                pass
            self.paths[name].append((layer, xy))
            for a, b in zip(xy, xy[1:]):
                g.add_seg(n, layer, a[0], a[1], b[0], b[1])
            for (x, y), l in run:
                i, j = g.cell(x, y)
                tree[l].add(i * ny + j)
        # the cells between the vertices belong to the tree too
        for l in range(2):
            pass
        for c, l in res:
            tree[l].add(c)
        self.add_pad_to_tree(tree, tgt, TOP if tgt.layer == 1 else BOT)


def octile(dx, dy):
    dx, dy = abs(dx), abs(dy)
    return G * (dx + dy) - G * 0.5858 * min(dx, dy)


def simplify(pts):
    """Drop points that lie on a straight line."""
    out = [pts[0]]
    for i in range(1, len(pts) - 1):
        a, b, c = out[-1], pts[i], pts[i + 1]
        if abs((b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0])) < 1e-6:
            continue
        out.append(b)
    out.append(pts[-1])
    return out


# ---------------------------------------------------------------------------------------------
# Result: tracks and vias as geometry, checks independent of the router's grid


class Copper:
    """What the router made: tracks (layer, x1, y1, x2, y2, net), vias (x, y, net)."""

    def __init__(self):
        self.tracks, self.vias = [], []


def collect(router):
    """Segments of every net, plus the stubs from pad centres to where a track starts in a pad."""
    b = router.board
    cu = Copper()
    for name, paths in router.paths.items():
        for layer, xy in paths:
            pts = list(xy)
            for end in (0, -1):
                p = pts[end]
                for pad in router.nets[name]:
                    if (TOP if pad.layer == 1 else BOT) == layer and abs(p[0] - pad.x) <= pad.w / 2 and \
                            abs(p[1] - pad.y) <= pad.h / 2:
                        if abs(p[0] - pad.x) + abs(p[1] - pad.y) > 0.3:
                            cu.tracks.append((layer, pad.x, pad.y, p[0], p[1], name))
                        break
            for a, c in zip(pts, pts[1:]):
                cu.tracks.append((layer, a[0], a[1], c[0], c[1], name))
        for (x, y) in router.out_vias[name]:
            cu.vias.append((x, y, name))
    split_junctions(cu)
    return cu


def split_junctions(cu):
    """A track that starts in the middle of another one of its net splits that one there."""
    changed = True
    while changed:
        changed = False
        pts = defaultdict(set)
        for (l, x1, y1, x2, y2, net) in cu.tracks:
            pts[(net, l)].update([(round(x1, 3), round(y1, 3)), (round(x2, 3), round(y2, 3))])
        for (x, y, net) in cu.vias:
            for l in (TOP, BOT):
                pts[(net, l)].add((round(x, 3), round(y, 3)))
        out = []
        for t in cu.tracks:
            l, x1, y1, x2, y2, net = t
            cut = None
            for (px, py) in pts[(net, l)]:
                if (abs(px - x1) < 0.01 and abs(py - y1) < 0.01) or (abs(px - x2) < 0.01 and abs(py - y2) < 0.01):
                    continue
                if seg_dist(np.array(px), np.array(py), x1, y1, x2, y2) < 0.02:
                    cut = (px, py)
                    break
            if cut:
                out.append((l, x1, y1, cut[0], cut[1], net))
                out.append((l, cut[0], cut[1], x2, y2, net))
                changed = True
            else:
                out.append(t)
        cu.tracks = out


def seg_seg_dist(a, b, c, d):
    """Distance between segments ab and cd."""
    def pt(p, q, r):
        dx, dy = r[0] - q[0], r[1] - q[1]
        L = dx * dx + dy * dy
        t = 0 if L == 0 else max(0, min(1, ((p[0] - q[0]) * dx + (p[1] - q[1]) * dy) / L))
        return math.hypot(p[0] - q[0] - t * dx, p[1] - q[1] - t * dy)

    def cross(p, q, r, s):
        def o(a_, b_, c_):
            return (b_[0] - a_[0]) * (c_[1] - a_[1]) - (b_[1] - a_[1]) * (c_[0] - a_[0])
        return o(p, q, r) * o(p, q, s) < 0 and o(r, s, p) * o(r, s, q) < 0
    if cross(a, b, c, d):
        return 0.0
    return min(pt(a, c, d), pt(b, c, d), pt(c, a, b), pt(d, a, b))


def seg_rect_dist(a, b, cx, cy, hw, hh):
    """Distance from segment ab to a rectangle (0 if they touch)."""
    # sample along the segment: fine enough for a clearance check at 1 mil
    L = math.hypot(b[0] - a[0], b[1] - a[1])
    n = max(2, int(L / 1.0) + 1)
    best = 1e9
    for i in range(n + 1):
        t = i / n
        x, y = a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])
        dx, dy = max(abs(x - cx) - hw, 0), max(abs(y - cy) - hh, 0)
        best = min(best, math.hypot(dx, dy))
    return best


def check(board, cu, clearance=DRC_CLR, verbose=True):
    """Every net connected, nothing too close, everything inside the outline. Returns problems."""
    problems = []
    nets = board.nets()
    pads = [p for p in board.pads.values() if p.net]
    # --- connectivity: union-find over pads, track ends, vias
    parent = {}

    def find(x):
        while parent.setdefault(x, x) != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    def union(a, b):
        parent[find(a)] = find(b)

    def pad_at(layer, x, y, net):
        for p in nets[net]:
            if (TOP if p.layer == 1 else BOT) == layer and abs(x - p.x) <= p.w / 2 + 0.01 and abs(y - p.y) <= p.h / 2 + 0.01:
                return ("pad",) + p.key
        return None

    nodes = defaultdict(list)   # (net, layer) -> [(point, id)]
    for i, (l, x1, y1, x2, y2, net) in enumerate(cu.tracks):
        union(("t", i), ("t", i))
        for pt in ((x1, y1), (x2, y2)):
            nodes[(net, l)].append((pt, ("t", i)))
            pp = pad_at(l, pt[0], pt[1], net)
            if pp:
                union(("t", i), pp)
        # crossing / touching other tracks of the same net on the same layer
    for (net, l), lst in nodes.items():
        for (p, a) in lst:
            for (q, b_) in lst:
                if abs(p[0] - q[0]) < 0.01 and abs(p[1] - q[1]) < 0.01:
                    union(a, b_)
    for k_, (x, y, net) in enumerate(cu.vias):
        for l in (TOP, BOT):
            for (p, a) in nodes[(net, l)]:
                if abs(p[0] - x) < 0.01 and abs(p[1] - y) < 0.01:
                    union(("v", k_), a)
    for net, plist in nets.items():
        roots = {find(("pad",) + p.key) for p in plist}
        if len(roots) > 1:
            problems.append(f"open net {net}: {len(roots)} parts")
    # --- clearance
    items = []   # (kind, net, layer, data)
    for (l, x1, y1, x2, y2, net) in cu.tracks:
        items.append(("t", net, l, (x1, y1, x2, y2)))
    for (x, y, net) in cu.vias:
        items.append(("v", net, 2, (x, y)))
    for p in pads:
        items.append(("p", p.net, TOP if p.layer == 1 else BOT, (p.x, p.y, p.w / 2, p.h / 2)))

    def bbox(it):
        k_, net, l, d = it
        if k_ == "t":
            return min(d[0], d[2]) - TW / 2, min(d[1], d[3]) - TW / 2, max(d[0], d[2]) + TW / 2, max(d[1], d[3]) + TW / 2
        if k_ == "v":
            return d[0] - VIA_R, d[1] - VIA_R, d[0] + VIA_R, d[1] + VIA_R
        return d[0] - d[2], d[1] - d[3], d[0] + d[2], d[1] + d[3]

    boxes = [bbox(it) for it in items]
    # spatial hash
    cellsz = 100.0
    hashed = defaultdict(list)
    for idx, bx in enumerate(boxes):
        for i in range(int((bx[0] - clearance) // cellsz), int((bx[2] + clearance) // cellsz) + 1):
            for j in range(int((bx[1] - clearance) // cellsz), int((bx[3] + clearance) // cellsz) + 1):
                hashed[(i, j)].append(idx)
    seen = set()
    worst = 1e9
    for cell, idxs in hashed.items():
        for a in idxs:
            for c in idxs:
                if a >= c or (a, c) in seen:
                    continue
                seen.add((a, c))
                A, B = items[a], items[c]
                if A[1] == B[1]:
                    continue
                if A[2] != B[2] and 2 not in (A[2], B[2]):
                    continue
                if A[0] == "p" and B[0] == "p":
                    continue
                bxa, bxb = boxes[a], boxes[c]
                if bxa[0] > bxb[2] + clearance or bxb[0] > bxa[2] + clearance or bxa[1] > bxb[3] + clearance or bxb[1] > bxa[3] + clearance:
                    continue
                dist = item_dist(A, B)
                worst = min(worst, dist)
                if dist < clearance - 1e-6:
                    problems.append(f"clearance {dist:.1f} mil: {A[0]} {A[1]} / {B[0]} {B[1]} near ({bxa[0]:.0f}, {bxa[1]:.0f})")
    # --- outline
    x0, y0, x1, y1 = board.outline
    for it, bx in zip(items, boxes):
        if it[0] != "p" and (bx[0] < x0 + 10 or bx[1] < y0 + 10 or bx[2] > x1 - 10 or bx[3] > y1 - 10):
            problems.append(f"{it[0]} {it[1]} too close to the board edge at ({bx[0]:.0f}, {bx[1]:.0f})")
    if verbose:
        print(f"check: {len(cu.tracks)} tracks, {len(cu.vias)} vias, tightest clearance {worst:.1f} mil, {len(problems)} problems")
    return problems


def item_dist(A, B):
    """Copper-to-copper distance of two items (track, via, pad)."""
    ka, kb = A[0], B[0]
    if ka > kb:
        A, B = B, A
        ka, kb = kb, ka
    # order: p < t < v
    def as_seg(it):
        d = it[3]
        return ((d[0], d[1]), (d[2], d[3]))
    if ka == "t" and kb == "t":
        return seg_seg_dist(*as_seg(A), *as_seg(B)) - TW
    if ka == "t" and kb == "v":
        s = as_seg(A)
        return seg_seg_dist(s[0], s[1], B[3], B[3]) - TW / 2 - VIA_R
    if ka == "p" and kb == "t":
        s = as_seg(B)
        return seg_rect_dist(s[0], s[1], *A[3]) - TW / 2
    if ka == "p" and kb == "v":
        cx, cy, hw, hh = A[3]
        x, y = B[3]
        return math.hypot(max(abs(x - cx) - hw, 0), max(abs(y - cy) - hh, 0)) - VIA_R
    if ka == "v" and kb == "v":
        return math.hypot(A[3][0] - B[3][0], A[3][1] - B[3][1]) - VIA_D
    return 1e9


# ---------------------------------------------------------------------------------------------
# Writing the copper into the project


def emit_copper(prj, board, cu, ids):
    pcb = board.pcb
    for (layer, x1, y1, x2, y2, net) in cu.tracks:
        rec = Rec({"type": "LINE", "ticket": ids.tick(), "id": ids.hexid()},
                  {"partitionId": "", "groupId": 0, "netName": net, "layerId": 1 if layer == TOP else 2,
                   "startX": round(x1, 4), "startY": round(y1, 4), "endX": round(x2, 4), "endY": round(y2, 4),
                   "width": TW, "locked": False, "zIndex": -1}, "", pcb)
        rec.dirty = True
        pcb.recs.append(rec)
    for (x, y, net) in cu.vias:
        rec = Rec({"type": "VIA", "ticket": ids.tick(), "id": ids.hexid()},
                  {"partitionId": "", "groupId": 0, "netName": net, "ruleName": "", "centerX": round(x, 4),
                   "centerY": round(y, 4), "holeDiameter": VIA_H, "viaDiameter": VIA_D, "viaType": "NORMAL",
                   "topSolderExpansion": None, "bottomSolderExpansion": None, "locked": False,
                   "unusedInnerLayers": []}, "", pcb)
        rec.dirty = True
        pcb.recs.append(rec)
    last = max(r.head.get("ticket", 0) for r in pcb.recs)
    pcb.head_rec.head["ticket"] = max(pcb.head_rec.head["ticket"], last + 1)
    pcb.head_rec.dirty = True
    pcb.head_rec.body = pcb.head_rec.body


# ---------------------------------------------------------------------------------------------
# Drawing


def render(board, cu, path):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    from matplotlib.patches import Rectangle, Circle
    fig, axes = plt.subplots(1, 2, figsize=(22, 20))
    col = {TOP: "#d02020", BOT: "#2050d0"}
    palette = {}
    for ax, layer in zip(axes, (TOP, BOT)):
        ax.set_aspect("equal")
        x0, y0, x1, y1 = board.outline
        ax.add_patch(Rectangle((x0, y0), x1 - x0, y1 - y0, fill=False, ec="k"))
        for p in board.pads.values():
            on = TOP if p.layer == 1 else BOT
            ax.add_patch(Rectangle((p.x - p.w / 2, p.y - p.h / 2), p.w, p.h, fc=col[on] if on == layer else "none",
                                   ec="#888" if on != layer else "k", alpha=0.35 if on == layer else 0.5, lw=0.5))
        for (l, ax1, ay1, ax2, ay2, net) in cu.tracks:
            if l == layer:
                ax.plot([ax1, ax2], [ay1, ay2], color=col[layer], lw=1.6, solid_capstyle="round")
        for (x, y, net) in cu.vias:
            ax.add_patch(Circle((x, y), VIA_R, fc="#40a040", ec="k", lw=0.4))
        for des, pt in board.parts.items():
            if des.startswith(("U", "J", "C", "R")):
                ax.text(pt["x"], pt["y"], des, fontsize=8, ha="center")
        ax.set_title("Top layer" if layer == TOP else "Bottom layer (seen from the top)")
        ax.set_xlim(x0 - 20, x1 + 20)
        ax.set_ylim(y0 - 20, y1 + 20)
    fig.tight_layout()
    fig.savefig(path, dpi=110)


# ---------------------------------------------------------------------------------------------
# Putting it together


def build(src):
    prj = Project(src)
    board = Board(prj)
    ids = Ids(prj)
    cells = switch_cells(board)
    log = []
    edit_schematic(prj, cells, ids, log)
    edit_pcb_nets(prj, board, cells, ids, log)
    return prj, board, ids, cells, log


def solve(board, rounds=8):
    """Route every net; a net that fails is moved to the front and the board is routed again."""
    names = sorted(board.nets())
    last = ["SDA", "SCL", "KP_INT", "KP_RESET", "PWR_ON", "3V3", "GND"]
    order = [n for n in names if n not in last] + [n for n in last if n in names]
    for rnd in range(rounds):
        router = Router(board)
        for name in order:
            router.route_net(name)
        failed = sorted({n for n, _ in router.failed})
        print(f"round {rnd}: failed {failed}", flush=True)
        if not failed:
            return router
        order = failed + [n for n in order if n not in failed]
    return router


def print_table(cells):
    grid = defaultdict(dict)
    for des, (leg, r, c) in cells.items():
        if r is not None:
            grid[r][c] = leg
    for r in range(ROWS):
        print(f"R{r:<2}", " ".join(f"{grid[r].get(c, '.'):>6}" for c in range(COLS_SMALL)))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--table", action="store_true", help="print the new matrix and stop")
    ap.add_argument("--render", metavar="PNG", help="draw both layers")
    ap.add_argument("project", help="the one-board keypad's EasyEDA Pro project")
    ap.add_argument("--out", help="where to write (default: next to the project, `_rerouted`)")
    a = ap.parse_args()
    a.out = a.out or beside(a.project, "_rerouted")
    prj, board, ids, cells, log = build(a.project)
    print_table(cells)
    if a.table:
        return
    router = solve(board)
    cu = collect(router)
    problems = check(board, cu)
    for p in problems:
        print("PROBLEM", p)
    if a.render:
        render(board, cu, a.render)
    if problems:
        sys.exit("not written: the board has problems")
    emit_copper(prj, board, cu, Ids(prj))
    prj.write(a.out)
    length = sum(math.hypot(t[3] - t[1], t[4] - t[2]) for t in cu.tracks)
    print(f"wrote {a.out}: {len(cu.tracks)} tracks ({length / 39.37:.0f} mm), {len(cu.vias)} vias")


if __name__ == "__main__":
    main()
