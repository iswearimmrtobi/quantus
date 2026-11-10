#!/usr/bin/env python3
"""Split the one-board keypad's EasyEDA Pro project into a top and a bottom board.

See firmware/docs/keypad-pcb.md for the two boards.

    python3 -I firmware/tools/keypad_split.py PROJECT   # writes PROJECT's _top and _bottom beside it

The cut runs at y -2810 mil, between the last function row and the first number row; the two
outlines together are the old one (73 x 115 mm). Every part keeps its place in the old coordinates,
except what moves to the bottom board (U2, C2 and J1). The output is the input of keypad_pcb.py,
which then sets up the 8 x 8 matrix of each board and routes it.

What it does, on top of keeping the parts of each board and dropping the rest:

- top: U1 A0 to GND (0x20), U1 INTB to KP_INT too, J2 for the cable to the bottom board.
- bottom: U2 and C2 become U1 and C1 and move under the number block, U1 INTA and INTB to KP_INT,
  J1 is the same connector as the top board's, at the cut, without PWR_ON.

J2 and the bottom board's J1 are the 8-pin JST SH of the main board connector, with its pinout, so
one cable type and one part serve all three; pin 8 (PWR_ON) stays open between the boards. All
copper goes: keypad_pcb.py routes each board anew.
"""
import copy
import json
import os
import re
import sys
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
sys.dont_write_bytecode = True
sys.path.insert(0, HERE)
import keypad_pcb as kp  # noqa: E402

CUT = -2810.0                   # mil, the edge between the two boards
NETPORT = "pid8a0e77bacb214e"   # schematic net port

# The bottom board: number rows at -3002 and -3415, chip on the back between them. C1 keeps the
# offset C2 had to U2, J1 sits at the top edge.
CHIP_Y = -3208.66
CAP_DY = -1025 - -728.3465
# Connector body: signal pads at y +76 of its origin, the fitting nails and the opening at -76..-112.
# Both connectors open towards the cut.
J2_AT = (1437.0079, CUT + 145, 0)       # top board, opening down
J1_BOTTOM_AT = (1437.0079, CUT - 145, 180)  # bottom board, opening up


def mcp_pin_elem(chip_id, pin):
    """The element id of a pin of the MCP23017 symbol, as NO_CONNECT flags name it."""
    return f"{chip_id}-e{2 + 4 * pin}"


def conn_pin_elem(conn_id, pin):
    return f"{conn_id}-e{1 + 4 * pin}"


class Sch:
    """One schematic page: components, their attributes, net ports with their wires."""

    def __init__(self, doc, prj):
        self.doc, self.prj = doc, prj
        self.title = kp.sch_page_title(doc)
        self.reindex()

    def reindex(self):
        d = self.doc
        live = [r for r in d.recs if r.body is not False]
        self.attrs = defaultdict(dict)
        self.attr_recs = defaultdict(list)
        for r in live:
            if r.type == "ATTR" and "parentId" in r.body:
                self.attrs[r.body["parentId"]][r.body["key"]] = r
                self.attr_recs[r.body["parentId"]].append(r)
        self.comps = {r.id: r for r in live if r.type == "COMPONENT"}
        self.lines = {r.id: r for r in live if r.type == "LINE"}
        self.wires = {r.id: r for r in live if r.type == "WIRE"}

    def designator(self, cid):
        a = self.attrs[cid].get("Designator")
        return a.body["value"] if a else None

    def comp(self, des):
        return next(cid for cid in self.comps if self.designator(cid) == des)

    def ports(self):
        """[(netport id, wire id, line id, (x, y) of the pin end)]"""
        out = []
        for cid, c in self.comps.items():
            if c.body["partId"] != NETPORT or "Name" not in self.attrs[cid] or self.designator(cid):
                continue
            x, y = c.body["x"], c.body["y"]
            for lid, ln in self.lines.items():
                b = ln.body
                if (b["endX"], b["endY"]) == (x, y):
                    out.append((cid, b["lineGroup"], lid, (b["startX"], b["startY"])))
                    break
        return out

    def port_at(self, xy):
        return next((p for p in self.ports() if p[3] == xy), None)

    def kill(self, rid):
        for r in self.doc.recs:
            if r.body is not False and (r.head.get("id") == rid or
                                        (r.type == "ATTR" and r.body.get("parentId") == rid)):
                r.body = False

    def kill_port(self, port):
        cid, wid, lid, _ = port
        for rid in (cid, wid, lid):
            self.kill(rid)

    def kill_comp(self, des):
        """A part, its net ports (wired to one of its pins) and its NO_CONNECT flags."""
        cid = self.comp(des)
        pins = self.pin_ends(cid)
        for p in self.ports():
            if p[3] in pins:
                self.kill_port(p)
        for r in self.doc.recs:
            if r.body is not False and r.type == "ATTR" and str(r.body.get("parentId", "")).startswith(cid + "-"):
                r.body = False
        self.kill(cid)
        self.reindex()

    def pin_ends(self, cid):
        """Pin ends of a part on the page, from the PIN records of its symbol."""
        c = self.comps[cid].body
        assert not c.get("rotation") and not c.get("isMirror"), c
        sym = self.attrs[cid]["Symbol"].body["value"]
        doc = next(d for d in self.prj.docs if d.kind == "SYMBOL" and d.uuid == sym)
        return {(c["x"] + r.body["x"], c["y"] + r.body["y"]) for r in doc.recs if r.type == "PIN"}

    def no_connect(self, elem):
        return next((r for r in self.doc.recs if r.body is not False and r.type == "ATTR"
                     and r.body.get("parentId") == elem and r.body.get("key") == "NO_CONNECT"), None)

    def drop_no_connect(self, elem):
        r = self.no_connect(elem)
        if r:
            r.body = False

    def add_no_connect(self, ids, elem, xy):
        if self.no_connect(elem):
            return
        self.doc.recs.append(kp.Rec({"type": "ATTR", "ticket": ids.tick(), "id": ids.hexid()},
                                    {"parentId": elem, "key": "NO_CONNECT", "x": xy[0], "y": xy[1],
                                     "value": "yes", "zIndex": self.top_z()}, "", self.doc))
        self.doc.recs[-1].dirty = True

    def top_z(self):
        return 1 + max((r.body.get("zIndex") or 0) for r in self.doc.recs
                       if r.body is not False and isinstance(r.body, dict) and isinstance(r.body.get("zIndex"), int))

    def clone_port(self, ids, port, name, dx=0, dy=0):
        """A copy of a net port and its wire, moved and renamed."""
        cid, wid, lid, _ = port
        mapping = {cid: ids.hexid(), wid: ids.hexid(), lid: ids.hexid()}
        new = []
        for r in self.doc.recs:
            if r.body is False:
                continue
            rid = r.head.get("id")
            parent = r.body.get("parentId") if r.type == "ATTR" else None
            if rid in mapping or parent in mapping:
                b = copy.deepcopy(r.body)
                for kx, ky in (("x", "y"), ("startX", "startY"), ("endX", "endY")):
                    if b.get(kx) is not None:
                        b[kx] += dx
                        b[ky] += dy
                if "lineGroup" in b:
                    b["lineGroup"] = mapping[b["lineGroup"]]
                if parent:
                    b["parentId"] = mapping[parent]
                if r.type == "ATTR" and b.get("key") == "Name":
                    b["value"] = name
                if isinstance(b.get("zIndex"), int) and r.type != "ATTR":
                    b["zIndex"] = self.top_z() + len(new)
                head = dict(r.head, ticket=ids.tick(), id=mapping.get(rid) or ids.hexid())
                head.pop("firstTicket", None)
                rec = kp.Rec(head, b, "", self.doc)
                rec.dirty = True
                new.append(rec)
        self.doc.recs.extend(new)
        self.reindex()

    def rename_port(self, port, name):
        rec = self.attrs[port[0]]["Name"]
        rec.body["value"] = name
        rec.dirty = True

    def set_designator(self, des, new):
        rec = self.attrs[self.comp(des)]["Designator"]
        rec.body["value"] = new
        rec.dirty = True

    def text(self, old_start, value):
        for r in self.doc.recs:
            if r.body is not False and r.type == "TEXT" and r.body["value"].startswith(old_start):
                r.body["value"] = value
                r.dirty = True
                return
        raise KeyError(old_start)


def mcp_pin_end(chip, pin):
    """Pin end of the MCP23017 symbol placed at chip (x, y): 1-14 on the left, 15-28 on the right."""
    x, y = chip
    if pin <= 14:
        return (x - 50, y - 75 + 10 * pin)
    return (x + 50, y + 215 - 10 * pin)


def conn_pin_end(conn, pin):
    x, y = conn
    return (x - 20, y - 40 + 10 * pin)


# ---------------------------------------------------------------------------------------------
# PCB


class Pcb:
    def __init__(self, prj):
        self.prj = prj
        self.doc = prj.doc("PCB")
        self.reindex()

    def reindex(self):
        live = [r for r in self.doc.recs if r.body is not False]
        self.comps = {}
        self.attrs = defaultdict(list)
        for r in live:
            if r.type == "ATTR":
                self.attrs[r.body["parentId"]].append(r)
        for r in live:
            if r.type == "COMPONENT":
                des = next(a.body["value"] for a in self.attrs[r.id] if a.body["key"] == "Designator")
                self.comps[des] = r
        self.pad_nets = defaultdict(dict)
        for r in live:
            if r.type == "PAD_NET":
                _, cid, num, _ = json.loads(r.id)
                self.pad_nets[cid][num] = r

    def kill(self, des):
        c = self.comps[des]
        c.body = False
        for a in self.attrs[c.id]:
            a.body = False
        for r in self.pad_nets[c.id].values():
            r.body = False
        self.reindex()

    def set_net(self, des, num, net):
        r = self.pad_nets[self.comps[des].id][str(num)]
        r.body["padNet"] = net
        r.dirty = True

    def set_designator(self, des, new):
        for a in self.attrs[self.comps[des].id]:
            if a.body["key"] == "Designator":
                a.body["value"] = new
                a.dirty = True
        self.reindex()

    def move(self, des, x, y, angle=None):
        """Move a part; its attribute texts keep their place relative to it (turned with it)."""
        c = self.comps[des]
        ox, oy, oa = c.body["x"], c.body["y"], c.body["angle"]
        turn = 0 if angle is None else (angle - oa) % 360
        c.body.update(x=x, y=y)
        if angle is not None:
            c.body["angle"] = angle
        c.dirty = True
        for a in self.attrs[c.id]:
            if a.body.get("x") is None:
                continue
            dx, dy = a.body["x"] - ox, a.body["y"] - oy
            if turn == 180:
                dx, dy = -dx, -dy
            a.body["x"], a.body["y"] = round(x + dx, 4), round(y + dy, 4)
            a.dirty = True

    def clone(self, ids, des, new_des, unique_id, channel):
        """A copy of a part with its pad nets and attributes, under a new designator."""
        c = self.comps[des]
        nid = ids.hexid()
        recs = []
        b = copy.deepcopy(c.body)
        b["attrs"] = dict(b["attrs"], **{"Unique ID": unique_id, "Channel ID": channel})
        recs.append(kp.Rec({"type": "COMPONENT", "ticket": ids.tick(), "id": nid}, b, "", self.doc))
        for num, r in self.pad_nets[c.id].items():
            _, _, n, elem = json.loads(r.id)
            rid = json.dumps(["PAD_NET", nid, n, elem], separators=(",", ":"))
            recs.append(kp.Rec({"type": "PAD_NET", "ticket": ids.tick(), "id": rid}, copy.deepcopy(r.body), "", self.doc))
        for a in self.attrs[c.id]:
            ab = copy.deepcopy(a.body)
            ab["parentId"] = nid
            if ab["key"] == "Designator":
                ab["value"] = new_des
            rid = nid + a.id[len(c.id):]
            recs.append(kp.Rec({"type": "ATTR", "ticket": ids.tick(), "id": rid}, ab, "", self.doc))
        for r in recs:
            r.dirty = True
        # after the original, so EasyEDA reads it in the same order
        i = self.doc.recs.index(c)
        self.doc.recs[i + 1:i + 1] = recs
        self.reindex()

    def keep_only(self, inside):
        """Drop the parts and free texts outside the board, and all copper."""
        for des, c in list(self.comps.items()):
            if not inside(c.body["y"]):
                self.kill(des)
        for r in self.doc.recs:
            if r.body is False:
                continue
            if r.type in ("LINE", "VIA") and r.body.get("netName"):
                r.body = False
            elif r.type == "STRING" and not inside(r.body["y"]):
                r.body = False
        self.reindex()

    def outline(self, y_top, y_bottom):
        r = next(r for r in self.doc.recs if r.type == "POLY" and r.body.get("polyType") == "BOARD_OUTLINE")
        p = r.body["path"]
        assert p[0] == "R", p
        p[2], p[4] = y_top, round(y_top - y_bottom, 4)
        r.dirty = True

    def prune_nets(self):
        """Drop the nets no pad has left (PWR_ON on the bottom board). ROW and COL nets are
        keypad_pcb.py's to keep or drop; it takes ROW0 as the template for new ones."""
        used = {r.body["padNet"] for cid in self.pad_nets for r in self.pad_nets[cid].values()}
        for r in self.doc.recs:
            if r.body is not False and r.type == "NET":
                name = json.loads(r.id)[1]
                if name and name not in used and not re.match(r"^(ROW|COL)\d+$", name):
                    r.body = False

    def channel_pages(self, page_map):
        for c in self.comps.values():
            ch = c.body["attrs"].get("Channel ID") or ""
            if ch.startswith("$") and "I" in ch:
                page, rest = ch[1:].split("I", 1)
                if int(page) in page_map:
                    c.body["attrs"]["Channel ID"] = f"${page_map[int(page)]}I{rest}"
                    c.dirty = True


# ---------------------------------------------------------------------------------------------
# Both boards


def drop_pages(prj, titles):
    """Remove schematic pages and number the rest again. Returns {old page no: new page no}."""
    pages = kp.sch_pages(prj)
    keep = [d for d in pages if kp.sch_page_title(d) not in titles]
    page_map = {pages.index(d) + 1: i + 1 for i, d in enumerate(keep)}
    prj.docs = [d for d in prj.docs if d in keep or d.kind != "SCH_PAGE"]
    for i, d in enumerate(keep):
        for r in d.recs:
            if r.type == "META":
                r.body["zIndex"] = i + 1
                r.dirty = True
            elif r.type == "ATTR" and r.body.get("key") == "@Page No":
                r.body["value"] = str(i + 1)
                r.dirty = True
            elif r.type == "ATTR" and r.body.get("key") == "@Page Count":
                r.body["value"] = str(len(keep))
                r.dirty = True
    return page_map


def page(prj, title):
    return Sch(next(d for d in kp.sch_pages(prj) if kp.sch_page_title(d) == title), prj)


def set_title(prj, title):
    meta = json.loads(prj.files["project2.json"])
    meta["title"] = title
    prj.files["project2.json"] = json.dumps(meta, indent=2, ensure_ascii=False).encode("utf-8")
    board = prj.doc("BOARD")
    for r in board.recs:
        if r.type == "META":
            r.body["title"] = title
            r.dirty = True


def matrix_ports(sch, chip_des, pins, ids, free):
    """Clear the NC flags of the matrix pins the script will use, flag the free ones."""
    cid = sch.comp(chip_des)
    c = sch.comps[cid].body
    for pin in pins:
        end = mcp_pin_end((c["x"], c["y"]), pin)
        if pin in free:
            port = sch.port_at(end)
            if port:
                sch.kill_port(port)
                sch.reindex()
            sch.add_no_connect(ids, mcp_pin_elem(cid, pin), end)
        else:
            sch.drop_no_connect(mcp_pin_elem(cid, pin))


GPA = list(range(21, 29))   # GPA0-7: rows
GPB = list(range(1, 9))     # GPB0-7: columns


def make_top(src, dst):
    prj = kp.Project(src)
    ids = kp.Ids(prj)
    page_map = drop_pages(prj, {"number_btn"})

    m = page(prj, "matrix_input")
    m.kill_comp("U2")
    m.kill_comp("C2")
    u1 = m.comp("U1")
    ux, uy = m.comps[u1].body["x"], m.comps[u1].body["y"]
    m.rename_port(m.port_at(mcp_pin_end((ux, uy), 15)), "GND")          # A0: 0x20
    kp_int = m.port_at(mcp_pin_end((ux, uy), 20))
    m.drop_no_connect(mcp_pin_elem(u1, 19))
    m.clone_port(ids, kp_int, "KP_INT", dy=+10)                           # INTB
    # small unit: rows R0-R7, columns C0-C5 and C7; GPB6 stays free
    matrix_ports(m, "U1", GPA + GPB, ids, free={7})
    m.text("To the main board", "To the main board: J1, to the bottom board: J2, on the page connector.")
    m.text("U1 rows", "Top board: U1 at 0x20 (A0-A2 to GND); the big unit's board ties A0 to 3V3 for 0x21. "
                      "Rows on GPA0-7, columns on GPB0-7, keypad-pcb.md.")
    m.text("INTA of U1", "INTA and INTB both on KP_INT: IOCON.MIRROR and IOCON.ODR (open drain) on both chips. "
                         "The KP_INT and I2C pull-ups are on the main board.")

    con = page(prj, "connector")
    j1 = con.comp("J1")
    jx, jy = con.comps[j1].body["x"], con.comps[j1].body["y"]
    dx = 300
    add_sch_connector(con, ids, j1, "J2", dx, unique_id=next_unique(prj))
    j2 = con.comp("J2")
    pwr = con.port_at(conn_pin_end((jx + dx, jy), 8))
    con.kill_port(pwr)
    con.reindex()
    con.add_no_connect(ids, conn_pin_elem(j2, 8), conn_pin_end((jx + dx, jy), 8))
    con.text("Connector to the main board", "J1 to the main board, at the top edge; J2 to the bottom board, at the "
                                            "cut. Both JST SH 8 pins (SM08B-SRSS-TB) on the back.")
    con.text("Tabs 9 and 10", "Tabs 9 and 10 only hold them. J2 has J1's pinout, pin 8 open. Check the pin order "
                              "against the main board and the cable.")

    pcb = Pcb(prj)
    pcb.kill("U2")
    pcb.kill("C2")
    pcb.keep_only(lambda y: y > CUT)
    pcb.outline(0, CUT)
    pcb.set_net("U1", 15, "GND")
    pcb.set_net("U1", 19, "KP_INT")
    pcb.set_net("U1", 7, "")                                              # GPB6, free
    pcb.clone(ids, "J1", "J2", unique_id=con.attrs[j2]["Unique ID"].body["value"], channel="")
    pcb.move("J2", *J2_AT)
    pcb.set_net("J2", 8, "")
    pcb.channel_pages(page_map)
    pcb.prune_nets()
    set_title(prj, os.path.splitext(os.path.basename(dst))[0])
    prj.write(dst)


def make_bottom(src, dst):
    prj = kp.Project(src)
    ids = kp.Ids(prj)
    page_map = drop_pages(prj, {"soft_btn", "control_btn", "arrow_btn", "function_btn"})

    m = page(prj, "matrix_input")
    for des in ("U1", "C1", "R1"):
        m.kill_comp(des)
    u2 = m.comp("U2")
    ux, uy = m.comps[u2].body["x"], m.comps[u2].body["y"]
    # INTA and INTB were open on U2: a KP_INT port each, copied from the KP_RESET port (pin 18)
    reset = m.port_at(mcp_pin_end((ux, uy), 18))
    for pin in (19, 20):
        m.drop_no_connect(mcp_pin_elem(u2, pin))
        m.clone_port(ids, reset, "KP_INT", dy=(18 - pin) * 10)
    # rows R0-R3 on GPA0-3, columns C0-C4 on GPB0-4; the rest stays free
    matrix_ports(m, "U2", GPA + GPB, ids, free={25, 26, 27, 28, 6, 7, 8})
    m.set_designator("U2", "U1")
    m.set_designator("C2", "C1")
    m.text("To the main board", "To the top board: J1 on the page connector.")
    m.text("U1 rows", "Bottom board: U1 at 0x22 (A1 to 3V3, A0 and A2 to GND), on both units. "
                      "Rows on GPA0-3, columns on GPB0-4, keypad-pcb.md.")
    m.text("INTA of U1", "INTA and INTB both on KP_INT: IOCON.MIRROR and IOCON.ODR (open drain). "
                         "The KP_INT and I2C pull-ups are on the main board, KP_RESET's on the top board.")

    con = page(prj, "connector")
    j1 = con.comp("J1")
    c = con.comps[j1].body
    pwr = con.port_at(conn_pin_end((c["x"], c["y"]), 8))
    con.kill_port(pwr)
    con.reindex()
    con.add_no_connect(ids, conn_pin_elem(j1, 8), conn_pin_end((c["x"], c["y"]), 8))
    con.text("Connector to the main board", "J1 to the top board (its J2): JST SH 8 pins (SM08B-SRSS-TB), on the "
                                            "back at the cut.")
    con.text("Tabs 9 and 10", "Tabs 9 and 10 only hold it. Pinout of the main board connector, pin 8 open. "
                              "Check the pin order against the cable.")

    pcb = Pcb(prj)
    moving = {"U2", "C2", "J1"}
    cx = pcb.comps["U2"].body["x"]
    cap_dx = pcb.comps["C2"].body["x"] - cx
    for des, c in list(pcb.comps.items()):
        if des not in moving and c.body["y"] > CUT:
            pcb.kill(des)
    pcb.reindex()
    pcb.move("U2", cx, CHIP_Y)
    pcb.move("C2", cx + cap_dx, round(CHIP_Y + CAP_DY, 4))
    pcb.move("J1", *J1_BOTTOM_AT)
    pcb.keep_only(lambda y: y < CUT)
    pcb.outline(CUT, -4527.5591)
    pcb.set_designator("U2", "U1")
    pcb.set_designator("C2", "C1")
    for pin in (19, 20):
        pcb.set_net("U1", pin, "KP_INT")
    for pin in (25, 26, 27, 28, 6, 7, 8):
        pcb.set_net("U1", pin, "")
    pcb.set_net("J1", 8, "")
    pcb.channel_pages(page_map)
    pcb.prune_nets()
    set_title(prj, os.path.splitext(os.path.basename(dst))[0])
    prj.write(dst)


def next_unique(prj):
    n = 0
    for r in prj.all:
        if r.type == "ATTR" and isinstance(r.body, dict) and r.body.get("key") == "Unique ID":
            v = r.body.get("value") or ""
            if v.startswith("gge") and v[3:].isdigit():
                n = max(n, int(v[3:]))
    return f"gge{n + 1}"


def add_sch_connector(sch, ids, src_id, des, dx, unique_id):
    """Copy a connector with its net ports and NC flags dx to the right."""
    c = sch.comps[src_id]
    nid = ids.hexid()
    recs = []
    b = copy.deepcopy(c.body)
    b["x"] += dx
    b["zIndex"] = sch.top_z()
    recs.append(kp.Rec({"type": "COMPONENT", "ticket": ids.tick(), "id": nid}, b, "", sch.doc))
    for a in sch.attr_recs[src_id]:
        ab = copy.deepcopy(a.body)
        ab["parentId"] = nid
        if ab.get("x") is not None:
            ab["x"] += dx
        if ab["key"] == "Designator":
            ab["value"] = des
        elif ab["key"] == "Unique ID":
            ab["value"] = unique_id
        recs.append(kp.Rec({"type": "ATTR", "ticket": ids.tick(), "id": ids.hexid()}, ab, "", sch.doc))
    for r in sch.doc.recs:
        if r.body is not False and r.type == "ATTR" and str(r.body.get("parentId", "")).startswith(src_id + "-"):
            ab = copy.deepcopy(r.body)
            ab["parentId"] = nid + ab["parentId"][len(src_id):]
            ab["x"] += dx
            recs.append(kp.Rec({"type": "ATTR", "ticket": ids.tick(), "id": ids.hexid()}, ab, "", sch.doc))
    for r in recs:
        r.dirty = True
    sch.doc.recs.extend(recs)
    sch.reindex()
    pins = {conn_pin_end((c.body["x"], c.body["y"]), p) for p in range(1, 9)}
    for port in sch.ports():
        if port[3] in pins:
            sch.clone_port(ids, port, sch.attrs[port[0]]["Name"].body["value"], dx=dx)


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: keypad_split.py PROJECT (the one-board keypad's EasyEDA Pro project)")
    src = sys.argv[1]
    top, bottom = kp.beside(src, "_top"), kp.beside(src, "_bottom")
    make_top(src, top)
    make_bottom(src, bottom)
    print(f"wrote {top} and {bottom}")


if __name__ == "__main__":
    main()
