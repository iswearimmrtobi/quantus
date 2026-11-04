#!/usr/bin/env python3
"""Generates the engine test set into test/cases/<category>/<category>-NNN.json.

Usage:
    python3 test/generator/generate.py            # write the cases
    python3 test/generator/generate.py --check    # also run the consistency checks

The output is deterministic for a given SEED and GENERATOR_VERSION. Only the standard library is used.
"""

import argparse
import json
import random
import shutil
import sys
from fractions import Fraction
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from gens import (algebra, analysis, arithmetic, calculus, complex_numbers, functions, linear_systems, physics, syntax,  # noqa: E402
                  basen, distributions, geometry, matrix, statistics, verify)

SEED = 20261023
GENERATOR_VERSION = 1
PER_FILE = 1000

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "test" / "cases"
ORACLE = ROOT / "crates" / "quantus-core" / "tests" / "golden" / "units_symbols.json"

# category -> (id prefix, number of cases, weighted generators)
CATEGORIES = {
    "arithmetic": ("arith", 15000, arithmetic.GENERATORS),
    "functions": ("func", 10000, functions.GENERATORS),
    "equations": ("eq", 8000, calculus.EQUATIONS),
    "derivatives": ("deriv", 7000, calculus.DERIVATIVES),
    "integrals": ("int", 7000, calculus.INTEGRALS),
    "limits": ("lim", 8000, calculus.LIMITS),
    "algebra": ("alg", 20000, algebra.GENERATORS),
    "physics": ("phys", 22000, physics.GENERATORS),
    "syntax": ("syn", 3000, syntax.GENERATORS),
    "complex": ("cplx", 3000, complex_numbers.GENERATORS),
    "linear-systems": ("lgs", 2000, linear_systems.GENERATORS),
    # The modes added later, appended so the seeds of the categories above stay the same.
    "verify": ("ver", 1000, verify.GENERATORS),
    "base-n": ("bn", 1000, basen.GENERATORS),
    "matrix": ("mat", 1000, matrix.GENERATORS),
    "statistics": ("stat", 1000, statistics.GENERATORS),
    "distributions": ("dist", 1000, distributions.GENERATORS),
    "analysis": ("ana", 1000, analysis.GENERATORS),
    "geometry": ("geo", 1000, geometry.GENERATORS),
}


def weighted_schedule(rng, generators, n):
    """Exactly round(n * w / sum(w)) cases per generator, shuffled."""
    total = sum(w for w, _ in generators)
    counts = [n * w // total for w, _ in generators]
    for i in range(n - sum(counts)):
        counts[i % len(counts)] += 1
    schedule = [g for (_, g), k in zip(generators, counts) for _ in range(k)]
    rng.shuffle(schedule)
    return schedule


def generate_category(index, name, prefix, n, generators):
    from gens.common import pick_difficulty
    rng = random.Random(SEED * 100 + index)
    seen = set()
    cases = []
    for i, gen in enumerate(weighted_schedule(rng, generators, n)):
        for attempt in range(25):
            c = gen(rng, pick_difficulty(rng))
            if c["latex"] not in seen:
                break
        seen.add(c["latex"])
        c = {"id": f"{prefix}-{i + 1:06d}", "category": name, **c}
        cases.append(c)
    return cases


def write(name, cases):
    d = OUT / name
    d.mkdir(parents=True, exist_ok=True)
    files = []
    for k in range(0, len(cases), PER_FILE):
        chunk = cases[k:k + PER_FILE]
        path = d / f"{name}-{k // PER_FILE + 1:03d}.json"
        head = json.dumps({"category": name, "generator_version": GENERATOR_VERSION, "seed": SEED,
                           "count": len(chunk)}, ensure_ascii=False)[:-1]
        body = ",\n".join("  " + json.dumps(c, ensure_ascii=False) for c in chunk)
        path.write_text(f"{head}, \"cases\": [\n{body}\n]}}\n", encoding="utf-8")
        files.append(path)
    return files


def check(all_cases):
    errors = []
    ids = [c["id"] for c in all_cases]
    if len(ids) != len(set(ids)):
        errors.append("duplicate ids")
    expected = sum(n for _, n, _ in CATEGORIES.values())
    if len(all_cases) != expected:
        errors.append(f"expected {expected} cases, got {len(all_cases)}")
    for c in all_cases:
        e = c["expect"]
        try:
            if e["kind"] in ("number", "quantity"):
                _parse_value(e["value"])
                float(e["rel_tol"])
            elif e["kind"] == "solutions":
                [_parse_value(v) for v in e["values"]]
            elif e["kind"] == "complex":
                _parse_value(e["re"])
                _parse_value(e["im"])
                float(e["rel_tol"])
            elif e["kind"] == "complex-solutions":
                [(_parse_value(v["re"]), _parse_value(v["im"])) for v in e["values"]]
            elif e["kind"] == "algebra":
                for vals in e["restrictions"].values():
                    [Fraction(v) for v in vals]
            elif e["kind"] == "linear":
                values = [v for row in e["rows"] for v in row] + e.get("values", [])
                [(_parse_value(v["re"]), _parse_value(v["im"])) if isinstance(v, dict) else _parse_value(v) for v in values]
                if e["outcome"] not in ("unique", "none", "infinite") or len(e["unknowns"]) != len(e["rows"][0]) - 1:
                    errors.append(f"{c['id']}: bad linear expectation")
                if "request" not in c:
                    errors.append(f"{c['id']}: no request")
            elif e["kind"] == "truth":
                if not isinstance(e["value"], bool):
                    errors.append(f"{c['id']}: truth without true or false")
            elif e["kind"] == "basen":
                int(e["value"])
                if len(e["forms"]) != 3:
                    errors.append(f"{c['id']}: basen needs the three other forms")
            elif e["kind"] == "matrix":
                [Fraction(v) for row in e["rows"] for v in row]
                if len({len(row) for row in e["rows"]}) != 1:
                    errors.append(f"{c['id']}: matrix rows of different lengths")
            elif e["kind"] == "statistics":
                for group in ("values", "regression", "estimates"):
                    for value, tol in e.get(group, {}).values():
                        _parse_value(value)
                        float(tol)
            elif e["kind"] == "distribution":
                for value, tol in e["values"]:
                    _parse_value(value)
                    float(tol)
            elif e["kind"] in ("analysis", "tangent"):
                float(e["tol"])
            elif e["kind"] == "area":
                _parse_value(e["area"])
                _parse_value(e["signed"])
            elif e["kind"] == "table":
                [_parse_value(v) for row in e["rows"] for v in row if v is not None]
            elif e["kind"] == "geometry":
                for _, value, tol in e["values"]:
                    _parse_value(value)
                    float(tol)
                [_parse_value(x) for _, p in e["points"] for x in p]
            elif e["kind"] != "error":
                errors.append(f"{c['id']}: unknown kind {e['kind']}")
        except Exception as ex:  # noqa: BLE001
            errors.append(f"{c['id']}: bad expectation {e}: {ex}")
        if "api" in c and not isinstance(c.get("request"), dict):
            errors.append(f"{c['id']}: api {c['api']} without a request")
        if not isinstance(c["latex"], str):
            errors.append(f"{c['id']}: latex is not a string")
    errors += check_units()
    return errors


def _parse_value(s):
    if "/" in s:
        return Fraction(s)
    from decimal import Decimal
    return Decimal(s)


def check_units():
    """The generator's unit factors must agree with the oracle dump wherever it lists the symbol."""
    from gens.physics import ATOMS, PREFIXES, U
    oracle = {s["symbol"]: s for s in json.loads(ORACLE.read_text(encoding="utf-8"))["symbols"] if "symbol" in s}
    errors = []
    checked = 0
    for atom in ATOMS:
        for prefix in [""] + list(PREFIXES):
            sym = prefix + atom
            o = oracle.get(sym) or oracle.get(sym.replace("µ", "μ"))
            if not o or (o.get("prefix") or "") != prefix and prefix:
                continue
            if o.get("offset", "0") != "0" or o.get("piPower", 0):
                continue
            x = U(prefix, atom)
            if Fraction(o["factor"]) != x.factor:
                errors.append(f"unit {sym}: generator factor {x.factor}, oracle {o['factor']}")
            dims = {k.strip("[]"): int(v) for k, v in o["dimensionVector"].items()}
            if dims != x.dim:
                errors.append(f"unit {sym}: generator dimension {x.dim}, oracle {dims}")
            checked += 1
    if checked < 100:
        errors.append(f"only {checked} unit symbols could be checked against the oracle")
    print(f"unit table: {checked} symbols agree with the oracle" if not errors else "unit table: mismatches")
    return errors


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--only", help="generate a single category (for development)")
    args = ap.parse_args()

    if not args.only and OUT.exists():
        shutil.rmtree(OUT)
    all_cases = []
    for index, (name, (prefix, n, gens)) in enumerate(CATEGORIES.items()):
        if args.only and name != args.only:
            continue
        cases = generate_category(index, name, prefix, n, gens)
        files = write(name, cases)
        all_cases += cases
        gaps = sum(c["gap"] for c in cases)
        print(f"{name:12} {len(cases):6} cases in {len(files):3} files, {gaps} not-implemented")
    if args.check:
        errors = check(all_cases) if not args.only else check_units()
        for e in errors[:50]:
            print("CHECK:", e)
        if errors:
            sys.exit(1)
        print("checks passed")


if __name__ == "__main__":
    main()
