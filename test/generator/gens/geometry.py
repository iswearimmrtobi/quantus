"""Geometry mode: a scene of points, vectors, lines, planes, spheres and circles, and a task on it.

Every object has rational data, given in one of its forms (coordinates, through points, normal or
coordinate form), and the answer is computed here over `Fraction`: distances (exact roots at 80
digits), feet, image points, midpoints and intersection points exactly, angles with `math` (the
engine shows them to a few digits after `\\approx`), and how two objects lie. The request is what
the mode sends to `api::geometry`; `latex` describes the scene for the report.
"""

import math
from fractions import Fraction as F

from decimal import Decimal as D

from .common import PI, case, d_sqrt, dec, expect_error, level, num_latex, value_str

TOL = "1e-40"
TOL_ANGLE = "1e-5"


# --- vectors over Q -----------------------------------------------------------------------------

def sub(a, b):
    return [x - y for x, y in zip(a, b)]


def add(a, b):
    return [x + y for x, y in zip(a, b)]


def scale(a, s):
    return [s * x for x in a]


def dot(a, b):
    return sum((x * y for x, y in zip(a, b)), F(0))


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def zero(a):
    return all(x == 0 for x in a)


def sqrt_value(q):
    """√q as an exact Fraction when q is a square, else at 80 digits."""
    q = F(q)
    n, d = math.isqrt(q.numerator), math.isqrt(q.denominator)
    if n * n == q.numerator and d * d == q.denominator:
        return F(n, d)
    return d_sqrt(q)


def _d(x):
    return x if isinstance(x, D) else dec(x)


def over(a, b):
    """a / b, exact when both are fractions."""
    return a / b if isinstance(a, F) and isinstance(b, F) else _d(a) / _d(b)


def cos_between(u, v):
    return float(abs(dot(u, v))) / math.sqrt(float(dot(u, u)) * float(dot(v, v)))


def angle_degrees(cosine):
    return math.degrees(math.acos(max(-1.0, min(1.0, float(cosine)))))


# --- input --------------------------------------------------------------------------------------

def _c(rng, diff, top=None):
    top = top or [4, 6, 9, 12][level(diff)]
    if level(diff) >= 2 and rng.random() < 0.15:
        return F(rng.randint(-2 * top, 2 * top), 2)
    return F(rng.randint(-top, top))


def _vec(rng, diff, nonzero=True):
    while True:
        v = [_c(rng, diff, 4 + level(diff)) for _ in range(3)]
        if not (nonzero and zero(v)):
            return v


def _point(rng, diff):
    return [_c(rng, diff) for _ in range(3)]


def _cells(v):
    return [num_latex(x, comma=False) for x in v]


def _point_text(v):
    return "(" + "|".join(num_latex(x, comma=False) for x in v) + ")"


def point_object(rng, name, p):
    form = rng.choice(["cells", "text", "typed"])
    if form == "cells":
        return {"name": name, "kind": "point", "at": _cells(p)}
    if form == "text":
        return {"name": name, "kind": "point", "at": _point_text(p)}
    return {"name": name, "kind": "typed", "latex": f"{name}{_point_text(p)}"}


def line_object(rng, name, support, direction):
    if rng.random() < 0.5:
        return {"name": name, "kind": "line", "support": _cells(support), "direction": _cells(direction)}
    other = add(support, scale(direction, F(rng.choice([1, 2, -1, 3]))))
    return {"name": name, "kind": "lineThrough", "a": _point_text(support), "b": _point_text(other)}


def plane_object(rng, name, normal, d, diff):
    """The plane n·x = d in one of four forms."""
    form = rng.choice(["coordinate", "normal", "through", "parameter"])
    support = _plane_point(normal, d)
    if form == "coordinate":
        k = F(rng.choice([1, 1, 2, -1, 3]))
        return {"name": name, "kind": "planeCoordinate", "a": num_latex(k * normal[0], comma=False),
                "b": num_latex(k * normal[1], comma=False), "c": num_latex(k * normal[2], comma=False),
                "d": num_latex(k * d, comma=False)}
    if form == "normal":
        return {"name": name, "kind": "planeNormal", "support": _cells(support), "normal": _cells(normal)}
    u, v = _spanning(normal)
    if form == "through":
        return {"name": name, "kind": "planeThrough", "a": _point_text(support), "b": _point_text(add(support, u)),
                "c": _point_text(add(support, v))}
    return {"name": name, "kind": "plane", "support": _cells(support), "u": _cells(u), "v": _cells(v)}


def _plane_point(n, d):
    i = next(k for k in range(3) if n[k] != 0)
    p = [F(0)] * 3
    p[i] = d / n[i]
    return p


def _spanning(n):
    """Two independent vectors perpendicular to n."""
    candidates = [cross(n, e) for e in ([1, 0, 0], [0, 1, 0], [0, 0, 1])]
    candidates = [[F(x) for x in c] for c in candidates if not zero(c)]
    u = candidates[0]
    v = next(c for c in candidates[1:] if not zero(cross(u, c)))
    return u, v


def _plane(rng, diff):
    n = _vec(rng, diff)
    return n, dot(n, _point(rng, diff))


def expect_geometry(relation=None, values=(), points=()):
    """`values`: [name, value, tol]; `points`: [name, [x, y, z]] exactly."""
    e = {"kind": "geometry", "values": [[n, repr(v) if isinstance(v, float) else value_str(v), t] for n, v, t in values],
         "points": [[n, [value_str(x) for x in p]] for n, p in points]}
    if relation:
        e["relation"] = relation
    return e


def _case(sub, diff, objects, task, expect, description="", degree=True):
    names = ", ".join(o.get("name") or o.get("latex", "") for o in objects)
    latex = f"{names}: {task['kind']} " + " ".join(str(v) for k, v in task.items() if k != "kind")
    settings = {"mode": "geometry", "angleUnit": "degree" if degree else "radian"}
    c = case(sub, diff, latex, expect, description, settings=settings)
    c["api"] = "geometry"
    c["request"] = {"scene": {"objects": objects}, "task": task}
    return c


# --- tasks --------------------------------------------------------------------------------------

def point_point(rng, diff):
    a, b = _point(rng, diff), _point(rng, diff)
    while a == b:
        b = _point(rng, diff)
    objects = [point_object(rng, "A", a), point_object(rng, "B", b)]
    if rng.random() < 0.4:
        m = scale(add(a, b), F(1, 2))
        return _case("midpoint", diff, objects, {"kind": "midpoint", "a": "A", "b": "B"},
                     expect_geometry(points=[["M", m]]), "M = (A + B)/2")
    d = sqrt_value(dot(sub(b, a), sub(b, a)))
    return _case("distance-points", diff, objects, {"kind": "distance", "a": "A", "b": "B"},
                 expect_geometry(values=[["d", d, TOL]]), "|AB|")


def point_line(rng, diff):
    s, u, p = _point(rng, diff), _vec(rng, diff), _point(rng, diff)
    on = rng.random() < 0.2
    if on:
        p = add(s, scale(u, _c(rng, diff, 3)))
    on = zero(cross(sub(p, s), u))
    objects = [point_object(rng, "P", p), line_object(rng, "g", s, u)]
    t = dot(sub(p, s), u) / dot(u, u)
    foot = add(s, scale(u, t))
    task = rng.choice(["distance", "foot", "relation", "reflect"])
    if task == "distance":
        d = sqrt_value(dot(sub(p, foot), sub(p, foot)))
        return _case("point-line", diff, objects, {"kind": "distance", "a": "P", "b": "g"}, expect_geometry(values=[["d", d, TOL]]),
                     "distance from the foot")
    if task == "foot":
        return _case("point-line", diff, objects, {"kind": "foot", "point": "P", "onto": "g"}, expect_geometry(points=[["F", foot]]),
                     "the foot of the perpendicular")
    if task == "reflect":
        image = sub(scale(foot, F(2)), p)
        return _case("point-line", diff, objects, {"kind": "reflect", "point": "P", "mirror": "g"},
                     expect_geometry(points=[["P'", image]]), "P' = 2F − P")
    return _case("point-line", diff, objects, {"kind": "relation", "a": "P", "b": "g"},
                 expect_geometry(relation="on" if on else "off"), "P on g?")


def point_plane(rng, diff):
    n, d = _plane(rng, diff)
    p = _point(rng, diff)
    on = rng.random() < 0.2
    if on:
        p = _plane_point(n, d)
        u, v = _spanning(n)
        p = add(add(p, scale(u, _c(rng, diff, 2))), scale(v, _c(rng, diff, 2)))
    on = dot(n, p) == d
    objects = [point_object(rng, "P", p), plane_object(rng, "E", n, d, diff)]
    t = (d - dot(n, p)) / dot(n, n)
    foot = add(p, scale(n, t))
    task = rng.choice(["distance", "foot", "relation", "reflect"])
    if task == "distance":
        dist = over(abs(dot(n, p) - d), sqrt_value(dot(n, n)))
        return _case("point-plane", diff, objects, {"kind": "distance", "a": "P", "b": "E"},
                     expect_geometry(values=[["d", dist, TOL]]), "Hesse normal form")
    if task == "foot":
        return _case("point-plane", diff, objects, {"kind": "foot", "point": "P", "onto": "E"},
                     expect_geometry(points=[["F", foot]]), "the foot of the perpendicular")
    if task == "reflect":
        return _case("point-plane", diff, objects, {"kind": "reflect", "point": "P", "mirror": "E"},
                     expect_geometry(points=[["P'", sub(scale(foot, F(2)), p)]]), "P' = 2F − P")
    return _case("point-plane", diff, objects, {"kind": "relation", "a": "P", "b": "E"},
                 expect_geometry(relation="on" if on else "off"), "P in E?")


def line_line(rng, diff):
    s, u = _point(rng, diff), _vec(rng, diff)
    kind = rng.choice(["identical", "parallel", "intersecting", "intersecting", "skew", "skew"])
    if kind == "identical":
        t, v = add(s, scale(u, _c(rng, diff, 3))), scale(u, F(rng.choice([2, -1, 3, F(1, 2)])))
    elif kind == "parallel":
        t, v = _point(rng, diff), scale(u, F(rng.choice([1, -2, 3])))
        while zero(cross(sub(t, s), u)):
            t = _point(rng, diff)
    elif kind == "intersecting":
        meet = add(s, scale(u, _c(rng, diff, 3)))
        v = _vec(rng, diff)
        while zero(cross(u, v)):
            v = _vec(rng, diff)
        t = add(meet, scale(v, _c(rng, diff, 3)))
    else:
        v = _vec(rng, diff)
        t = _point(rng, diff)
        while zero(cross(u, v)) or dot(sub(t, s), cross(u, v)) == 0:
            v, t = _vec(rng, diff), _point(rng, diff)
    objects = [line_object(rng, "g", s, u), line_object(rng, "h", t, v)]
    values, points = [], []
    if kind == "parallel":
        c = cross(sub(t, s), u)
        values.append(["d", sqrt_value(dot(c, c) / dot(u, u)), TOL])
    elif kind == "intersecting":
        points.append(["S", meet])
        values.append(["\\varphi", angle_degrees(cos_between(u, v)), TOL_ANGLE])
    elif kind == "skew":
        n = cross(u, v)
        values.append(["d", over(abs(dot(sub(t, s), n)), sqrt_value(dot(n, n))), TOL])
    if rng.random() < 0.25 and kind in ("intersecting", "skew"):
        return _case("angle-lines", diff, objects, {"kind": "angle", "a": "g", "b": "h"},
                     expect_geometry(values=[["\\varphi", angle_degrees(cos_between(u, v)), TOL_ANGLE]]),
                     "angle between the directions")
    return _case(f"lines-{kind}", diff, objects, {"kind": "relation", "a": "g", "b": "h"},
                 expect_geometry(relation=kind, values=values, points=points), f"two lines, {kind}")


def line_plane(rng, diff):
    n, d = _plane(rng, diff)
    kind = rng.choice(["intersecting", "intersecting", "parallel", "contained"])
    u, v = _spanning(n)
    if kind == "intersecting":
        meet = add(add(_plane_point(n, d), scale(u, _c(rng, diff, 2))), scale(v, _c(rng, diff, 2)))
        w = _vec(rng, diff)
        while dot(w, n) == 0:
            w = _vec(rng, diff)
        s = add(meet, scale(w, _c(rng, diff, 3)))
    else:
        w = add(scale(u, _c(rng, diff, 2)), scale(v, _c(rng, diff, 2)))
        while zero(w):
            w = add(scale(u, _c(rng, diff, 2)), scale(v, _c(rng, diff, 2)))
        base = add(_plane_point(n, d), scale(u, _c(rng, diff, 2)))
        s = base if kind == "contained" else add(base, scale(n, F(rng.choice([1, -1, 2]))))
    objects = [line_object(rng, "g", s, w), plane_object(rng, "E", n, d, diff)]
    values, points = [], []
    if kind == "intersecting":
        points.append(["S", meet])
        values.append(["\\varphi", math.degrees(math.asin(min(1.0, cos_between(w, n)))), TOL_ANGLE])
    elif kind == "parallel":
        values.append(["d", over(abs(dot(n, s) - d), sqrt_value(dot(n, n))), TOL])
    return _case(f"line-plane-{kind}", diff, objects, {"kind": "relation", "a": "g", "b": "E"},
                 expect_geometry(relation=kind, values=values, points=points), f"line and plane, {kind}")


def plane_plane(rng, diff):
    n, d = _plane(rng, diff)
    kind = rng.choice(["identical", "parallel", "intersecting", "intersecting"])
    if kind == "identical":
        m, e = scale(n, F(rng.choice([2, -1, 3]))), None
        e = d * (m[0] / n[0] if n[0] else m[1] / n[1] if n[1] else m[2] / n[2])
    elif kind == "parallel":
        k = F(rng.choice([1, 2, -1]))
        m, e = scale(n, k), k * (d + rng.choice([1, -1, 2, 5]))
    else:
        m = _vec(rng, diff)
        while zero(cross(n, m)):
            m = _vec(rng, diff)
        e = dot(m, _point(rng, diff))
    objects = [plane_object(rng, "E", n, d, diff), plane_object(rng, "F", m, e, diff)]
    values = []
    if kind == "parallel":
        k = m[0] / n[0] if n[0] else m[1] / n[1] if n[1] else m[2] / n[2]
        values.append(["d", over(abs(e / k - d), sqrt_value(dot(n, n))), TOL])
    elif kind == "intersecting":
        values.append(["\\varphi", angle_degrees(cos_between(n, m)), TOL_ANGLE])
    return _case(f"planes-{kind}", diff, objects, {"kind": "relation", "a": "E", "b": "F"},
                 expect_geometry(relation=kind, values=values), f"two planes, {kind}")


def _sphere(rng, diff):
    return _point(rng, diff), F(rng.randint(1, 3 + 2 * level(diff)))


def sphere_object(rng, name, centre, r):
    if rng.random() < 0.7:
        return {"name": name, "kind": "sphere", "centre": _cells(centre), "radius": num_latex(r, comma=False)}
    # Centre and a point on it: (r, 0, 0) from the centre.
    on = add(centre, rng.choice([[r, 0, 0], [0, r, 0], [0, 0, -r]]))
    return {"name": name, "kind": "sphereThrough", "centre": _point_text(centre), "point": _point_text(on)}


def sphere_tasks(rng, diff):
    m, r = _sphere(rng, diff)
    kind = rng.choice(["point", "point", "plane", "line", "measure"])
    if kind == "measure":
        measure = rng.choice(["volume", "surface"])
        value = dec(F(4, 3) * r ** 3) * PI if measure == "volume" else dec(4 * r * r) * PI
        name = "V" if measure == "volume" else "O"
        return _case(f"sphere-{measure}", diff, [sphere_object(rng, "K", m, r)], {"kind": "measure", "object": "K", "measure": measure},
                     expect_geometry(values=[[name, value, TOL]]), "4/3 π r³ or 4π r²")
    if kind == "point":
        p = add(m, rng.choice([[r, 0, 0], [0, 0, r]])) if rng.random() < 0.25 else _point(rng, diff)
        dist2 = dot(sub(p, m), sub(p, m))
        relation = "on" if dist2 == r * r else "inside" if dist2 < r * r else "outside"
        values = [] if relation == "on" else [["d", abs(_d(sqrt_value(dist2)) - _d(r)), TOL]]
        objects = [sphere_object(rng, "K", m, r), point_object(rng, "P", p)]
        return _case("sphere-point", diff, objects, {"kind": "relation", "a": "K", "b": "P"},
                     expect_geometry(relation=relation, values=values), f"|MP| against r: {relation}")
    if kind == "plane":
        n = rng.choice([[F(1), F(0), F(0)], [F(0), F(1), F(0)], [F(0), F(0), F(1)], [F(1), F(2), F(2)], [F(2), F(-1), F(2)]])
        length = sqrt_value(dot(n, n))
        offset = F(rng.randint(0, int(r) + 2))
        d = dot(n, m) + offset * length
        relation = "circle" if offset < r else "tangent" if offset == r else "apart"
        objects = [sphere_object(rng, "K", m, r), plane_object(rng, "E", n, d, diff)]
        return _case("sphere-plane", diff, objects, {"kind": "relation", "a": "K", "b": "E"},
                     expect_geometry(relation=relation), f"distance {offset} against r = {r}")
    u = rng.choice([[F(1), F(0), F(0)], [F(0), F(1), F(0)], [F(0), F(0), F(1)]])
    offset = F(rng.randint(0, int(r) + 2))
    other = [k for k in range(3) if u[k] == 0][0]
    s = list(m)
    s[other] += offset
    relation = "secant" if offset < r else "tangent" if offset == r else "passant"
    objects = [sphere_object(rng, "K", m, r), line_object(rng, "g", s, u)]
    return _case("sphere-line", diff, objects, {"kind": "relation", "a": "K", "b": "g"},
                 expect_geometry(relation=relation), f"distance {offset} against r = {r}")


def circle_measures(rng, diff):
    centre, r = _point(rng, diff), F(rng.randint(1, 9))
    measure = rng.choice(["circumference", "area"])
    obj = {"name": "k", "kind": "circle", "centre": _cells(centre), "radius": num_latex(r, comma=False), "normal": None}
    if rng.random() < 0.5:
        obj["normal"] = _cells(_vec(rng, diff))
    value = dec(2 * r) * PI if measure == "circumference" else dec(r * r) * PI
    return _case(f"circle-{measure}", diff, [obj], {"kind": "measure", "object": "k", "measure": measure},
                 expect_geometry(values=[["U" if measure == "circumference" else "A", value, TOL]]), "2πr or πr²")


def errors(rng, diff):
    m, r = _sphere(rng, diff)
    kind = rng.choice(["angle", "unknown", "same"])
    if kind == "angle":
        objects = [point_object(rng, "A", _point(rng, diff)), sphere_object(rng, "K", m, r)]
        return _case("errors", diff, objects, {"kind": "angle", "a": "A", "b": "K"}, expect_error("does not work"), "no angle")
    if kind == "unknown":
        objects = [point_object(rng, "A", _point(rng, diff))]
        return _case("errors", diff, objects, {"kind": "distance", "a": "A", "b": "Z"}, expect_error("Z"), "an unknown name")
    p = _point(rng, diff)
    objects = [{"name": "g", "kind": "lineThrough", "a": _point_text(p), "b": _point_text(p)}]
    return _case("errors", diff, objects, {"kind": "measure", "object": "g", "measure": "length"}, expect_error(""),
                 "a line through one point twice")


GENERATORS = [
    (12, point_point),
    (14, point_line),
    (14, point_plane),
    (20, line_line),
    (14, line_plane),
    (10, plane_plane),
    (10, sphere_tasks),
    (3, circle_measures),
    (3, errors),
]
