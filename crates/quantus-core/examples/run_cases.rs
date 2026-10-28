//! Runs the generated test set in `test/cases` against the engine and writes a report.
//!
//! ```text
//! cargo run --release -p quantus-core --example run_cases -- [options]
//!   --cases DIR      case directory (default test/cases)
//!   --out DIR        report directory (default test/results)
//!   --filter TEXT    only cases whose id, category or category/subcategory starts with TEXT (repeatable)
//!   --jobs N         worker processes (default: available parallelism, at most 8)
//!   --timeout SECS   per-case time limit (default 5)
//!   --limit N        stop after N cases (after filtering)
//!   --fail-only      write only non-passing cases to results.jsonl (failures.jsonl always has just those)
//! ```
//!
//! The case format is described in `test/README.md`. Cases are evaluated in worker processes capped at 4 GiB of address
//! space, so a panic, a hang or a runaway allocation is reported as `PANIC` / `TIMEOUT` / `ERROR` instead of
//! stopping the run; a timed-out worker is killed and replaced.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use dashu::base::Abs;
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;
use serde_json::{json, Value};

use quantus_core::algebra::domain::{format_restrictions_plain, DomainRestriction};
use quantus_core::algebra::nested::split_fraction;
use quantus_core::algebra::pipeline::{collect_variables, simplify_node_with, PipelineConfig};
use quantus_core::algebra::poly::Poly;
use quantus_core::algebra::verify::eval_rational;
use quantus_core::eval::{Evaluator, LatexCalculator, Variables};
use quantus_core::latex::{parser, Node};
use quantus_core::num::Decimal;
use quantus_core::settings::{Mode, Settings};
use quantus_core::units::BaseDim;
use quantus_core::PhysicalQuantity;

const PRECISION: usize = 50;

struct Options {
    cases: PathBuf,
    out: PathBuf,
    filters: Vec<String>,
    jobs: usize,
    timeout: Duration,
    limit: Option<usize>,
    fail_only: bool,
}

fn options() -> Options {
    let mut o = Options {
        cases: PathBuf::from("test/cases"),
        out: PathBuf::from("test/results"),
        filters: Vec::new(),
        jobs: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(8),
        timeout: Duration::from_secs(5),
        limit: None,
        fail_only: false,
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| panic!("{a} needs a value"));
        match a.as_str() {
            "--cases" => o.cases = value().into(),
            "--out" => o.out = value().into(),
            "--filter" => o.filters.push(value()),
            "--jobs" => o.jobs = value().parse().expect("--jobs"),
            "--timeout" => o.timeout = Duration::from_secs_f64(value().parse().expect("--timeout")),
            "--limit" => o.limit = Some(value().parse().expect("--limit")),
            "--fail-only" => o.fail_only = true,
            other => panic!("unknown option {other}"),
        }
    }
    o
}

// --- outcome ----------------------------------------------------------------------------------

#[derive(Clone)]
struct Outcome {
    /// PASS, FAIL, ERROR, UNEXPECTED_OK, PANIC, TIMEOUT or GENERATOR (the case itself is wrong).
    status: &'static str,
    /// For FAIL: wrong-value, wrong-dimension, not-reduced, restrictions, solutions, wrong-error, ...
    reason: String,
    detail: String,
    actual: String,
    note: String,
}

impl Outcome {
    fn pass(actual: String) -> Self {
        Outcome { status: "PASS", reason: String::new(), detail: String::new(), actual, note: String::new() }
    }
    fn fail(reason: &str, detail: String, actual: String) -> Self {
        Outcome { status: "FAIL", reason: reason.into(), detail, actual, note: String::new() }
    }
    fn error(message: String) -> Self {
        Outcome { status: "ERROR", reason: String::new(), detail: message, actual: String::new(), note: String::new() }
    }
    fn label(&self) -> String {
        if self.reason.is_empty() { self.status.to_string() } else { format!("{}({})", self.status, self.reason) }
    }
}

// --- numbers ----------------------------------------------------------------------------------

fn parse_rational(text: &str) -> Option<RBig> {
    if let Some((n, d)) = text.split_once('/') {
        let n: IBig = n.trim().parse().ok()?;
        let d: UBig = d.trim().parse().ok()?;
        return Some(RBig::from_parts(n, d));
    }
    Decimal::parse(text).map(|d| d.to_rational())
}

/// Relative tolerance; a zero expectation is compared absolutely. With an exact tolerance (1e-45,
/// the engine's 50 digits) zero must come out as exactly zero: `\sin \pi` is 0, not 1.2·10^-97.
fn close(actual: &RBig, expected: &RBig, tol: &RBig) -> bool {
    let diff = (actual - expected).abs();
    if expected.is_zero() {
        if *tol <= exact_tolerance() {
            return actual.is_zero();
        }
        diff <= *tol
    } else {
        diff <= tol * expected.clone().abs()
    }
}

fn exact_tolerance() -> RBig {
    RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(45))
}

fn show(r: &RBig) -> String {
    let d = Decimal::from_rational(r, 20);
    d.to_plain_string()
}

trait FromRational {
    fn from_rational(r: &RBig, digits: usize) -> Decimal;
}

impl FromRational for Decimal {
    fn from_rational(r: &RBig, digits: usize) -> Decimal {
        quantus_core::num::Value::Rat(r.clone()).decimal(digits).unwrap_or_else(|_| Decimal::from_i64(0))
    }
}

fn dim_letter(d: BaseDim) -> &'static str {
    match d {
        BaseDim::Length => "L",
        BaseDim::Mass => "M",
        BaseDim::Time => "T",
        BaseDim::Current => "I",
        BaseDim::Temperature => "Θ",
        BaseDim::Amount => "N",
        BaseDim::Luminous => "J",
    }
}

fn dims_of(q: &PhysicalQuantity) -> BTreeMap<String, i64> {
    q.dimension()
        .exponents()
        .into_iter()
        .filter(|(_, e)| *e != 0)
        .map(|(d, e)| (dim_letter(d).to_string(), e as i64))
        .collect()
}

fn expected_dims(e: &Value) -> BTreeMap<String, i64> {
    e.get("dim")
        .and_then(Value::as_object)
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.as_i64().unwrap_or(0))).collect())
        .unwrap_or_default()
}

fn fmt_dims(d: &BTreeMap<String, i64>) -> String {
    if d.is_empty() {
        return "1".into();
    }
    d.iter().map(|(k, e)| if *e == 1 { k.clone() } else { format!("{k}^{e}") }).collect::<Vec<_>>().join("·")
}

/// The quantity's value in the coherent SI unit of its dimension (°C becomes K).
fn si_value(q: &PhysicalQuantity, displayed: &Decimal) -> Result<RBig, String> {
    let conversion = q.unit.system_conversion().map_err(|e| e.to_string())?;
    Ok(conversion.apply(&displayed.to_rational()))
}

// --- evaluation -------------------------------------------------------------------------------

fn settings_of(case: &Value) -> Settings {
    let mut s = Settings::default();
    if let Some(m) = case.get("settings").and_then(Value::as_object) {
        if let Some(b) = m.get("a_and_y_as_variables").and_then(Value::as_bool) {
            s.a_and_y_as_variables = b;
        }
        if let Some(b) = m.get("letter_products").and_then(Value::as_bool) {
            s.letter_products = b;
        }
        if let Some(b) = m.get("extra_unit_letters").and_then(Value::as_bool) {
            s.extra_unit_letters = b;
        }
        if let Some(mode) = m.get("mode").and_then(Value::as_str) {
            s.mode = match mode {
                "algebra" => Mode::Algebra,
                "physics" => Mode::Physics,
                "imaginary" => Mode::Imaginary,
                "electronics" => Mode::Electronics,
                other => panic!("unknown mode {other:?}"),
            };
        }
    }
    s
}

fn run_case(case: &Value) -> Outcome {
    let latex = case["latex"].as_str().unwrap_or_default();
    let expect = &case["expect"];
    let settings = settings_of(case);
    let calculator = LatexCalculator::new(PRECISION).with_settings(settings.clone());
    let vars = Variables::new();
    let result = calculator.evaluate(latex, &vars);
    let kind = expect["kind"].as_str().unwrap_or_default();

    if kind == "error" {
        let contains = expect["contains"].as_str().unwrap_or_default();
        return match result {
            Err(e) if e.to_string().contains(contains) => Outcome::pass(format!("error: {e}")),
            Err(e) => Outcome::fail("wrong-error", format!("expected an error containing '{contains}'"), format!("error: {e}")),
            Ok(calc) => Outcome {
                status: "UNEXPECTED_OK",
                reason: String::new(),
                detail: "expected an error".into(),
                actual: calc.latex,
                note: String::new(),
            },
        };
    }
    let calc = match result {
        Ok(c) => c,
        Err(e) => return Outcome::error(e.to_string()),
    };
    let tol = expect["rel_tol"].as_str().and_then(parse_rational).unwrap_or(RBig::ZERO);

    match kind {
        "number" | "quantity" => {
            if let Some(w) = &calc.warning {
                return Outcome::fail("not-evaluated", w.clone(), calc.latex);
            }
            let Some(expected) = expect["value"].as_str().and_then(parse_rational) else {
                return generator("unparsable expected value");
            };
            let want_dims = expected_dims(expect);
            let got_dims = dims_of(&calc.quantity);
            if want_dims != got_dims {
                return Outcome::fail(
                    "wrong-dimension",
                    format!("expected dimension {}, got {}", fmt_dims(&want_dims), fmt_dims(&got_dims)),
                    calc.latex,
                );
            }
            let actual = if kind == "quantity" {
                match si_value(&calc.quantity, &calc.value) {
                    Ok(v) => v,
                    Err(e) => return Outcome::fail("unit-not-convertible", e, calc.latex),
                }
            } else {
                calc.value.to_rational()
            };
            if !close(&actual, &expected, &tol) {
                return Outcome::fail(
                    "wrong-value",
                    format!("expected {}{}, got {}", show(&expected), if kind == "quantity" { " (SI)" } else { "" }, show(&actual)),
                    calc.latex,
                );
            }
            let mut out = Outcome::pass(calc.latex);
            if let Some(unit) = expect.get("unit").and_then(Value::as_str) {
                if calc.unit != unit {
                    out.note = format!("shown in {} instead of {unit}", calc.unit);
                }
            }
            out
        }
        "complex" => {
            let parse = |key: &str| expect[key].as_str().and_then(parse_rational);
            let (Some(re), Some(im)) = (parse("re"), parse("im")) else {
                return generator("unparsable expected complex value");
            };
            let got_im = calc.imaginary.as_ref().map(|d| d.to_rational()).unwrap_or(RBig::ZERO);
            let got_re = calc.value.to_rational();
            if close(&got_re, &re, &tol) && close(&got_im, &im, &tol) {
                Outcome::pass(calc.latex)
            } else {
                Outcome::fail(
                    "wrong-value",
                    format!("expected {} + {}i, got {} + {}i", show(&re), show(&im), show(&got_re), show(&got_im)),
                    calc.latex,
                )
            }
        }
        "complex-solutions" => check_complex_solutions(latex, &settings, expect, &tol, calc.latex),
        "solutions" => check_solutions(latex, &settings, expect, &tol, calc.latex),
        "algebra" => check_algebra(latex, &settings, expect, calc.result),
        _ => generator("unknown expectation kind"),
    }
}

fn generator(message: &str) -> Outcome {
    Outcome { status: "GENERATOR", reason: String::new(), detail: message.into(), actual: String::new(), note: String::new() }
}

fn check_solutions(latex: &str, settings: &Settings, expect: &Value, tol: &RBig, shown: String) -> Outcome {
    let formula = match parser::parse_with(latex, &settings.parse_options()) {
        Ok(f) => f,
        Err(e) => return Outcome::error(e.to_string()),
    };
    let vars = Variables::new();
    let solutions = match Evaluator::new(&vars).with_precision(PRECISION).solve(&formula.expression) {
        Ok(s) => s,
        Err(e) => return Outcome::error(e.to_string()),
    };
    let want_dims = expected_dims(expect);
    let mut got = Vec::new();
    for q in &solutions.values {
        let q = match q.simplify() {
            Ok(q) => q,
            Err(e) => return Outcome::error(e.to_string()),
        };
        if dims_of(&q) != want_dims {
            return Outcome::fail(
                "wrong-dimension",
                format!("expected dimension {}, got {}", fmt_dims(&want_dims), fmt_dims(&dims_of(&q))),
                shown,
            );
        }
        let value = match q.decimal(PRECISION) {
            Ok(v) => v,
            Err(e) => return Outcome::error(e.to_string()),
        };
        match si_value(&q, &value) {
            Ok(v) => got.push(v),
            Err(e) => return Outcome::fail("unit-not-convertible", e, shown),
        }
    }
    let want: Vec<RBig> = expect["values"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().and_then(parse_rational)).collect())
        .unwrap_or_default();
    let dedup = |v: Vec<RBig>| {
        let mut out: Vec<RBig> = Vec::new();
        for x in v {
            if !out.iter().any(|y| close(&x, y, tol)) {
                out.push(x);
            }
        }
        out
    };
    let (got, want) = (dedup(got), dedup(want));
    let missing: Vec<String> = want.iter().filter(|w| !got.iter().any(|g| close(g, w, tol))).map(show).collect();
    let extra: Vec<String> = got.iter().filter(|g| !want.iter().any(|w| close(g, w, tol))).map(show).collect();
    if missing.is_empty() && extra.is_empty() {
        Outcome::pass(shown)
    } else {
        Outcome::fail("solutions", format!("missing [{}], unexpected [{}]", missing.join(", "), extra.join(", ")), shown)
    }
}

/// The roots of an equation as (re, im) pairs, compared with the expected ones in any order.
fn check_complex_solutions(latex: &str, settings: &Settings, expect: &Value, tol: &RBig, shown: String) -> Outcome {
    let formula = match parser::parse_with(latex, &settings.parse_options()) {
        Ok(f) => f,
        Err(e) => return Outcome::error(e.to_string()),
    };
    let vars = Variables::new();
    let evaluator = Evaluator::new(&vars).with_precision(PRECISION).with_mode(settings.mode);
    let solutions = match evaluator.solve(&formula.expression) {
        Ok(s) => s,
        Err(e) => return Outcome::error(e.to_string()),
    };
    let part = |value: quantus_core::num::Value| -> Result<RBig, String> {
        value.decimal(PRECISION).map(|d| d.to_rational()).map_err(|e| e.to_string())
    };
    let mut got = Vec::new();
    for q in &solutions.values {
        let Some((re, im)) = q.value.re_im() else { return Outcome::error("a root is not of the form a + bi".into()) };
        match (part(re), part(im)) {
            (Ok(re), Ok(im)) => got.push((re, im)),
            (Err(e), _) | (_, Err(e)) => return Outcome::error(e),
        }
    }
    let want: Vec<(RBig, RBig)> = expect["values"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| Some((parse_rational(v["re"].as_str()?)?, parse_rational(v["im"].as_str()?)?)))
                .collect()
        })
        .unwrap_or_default();
    let same = |x: &(RBig, RBig), y: &(RBig, RBig)| close(&x.0, &y.0, tol) && close(&x.1, &y.1, tol);
    let missing: Vec<String> =
        want.iter().filter(|w| !got.iter().any(|g| same(g, w))).map(|w| format!("{} + {}i", show(&w.0), show(&w.1))).collect();
    let extra: Vec<String> =
        got.iter().filter(|g| !want.iter().any(|w| same(g, w))).map(|g| format!("{} + {}i", show(&g.0), show(&g.1))).collect();
    if missing.is_empty() && extra.is_empty() {
        Outcome::pass(shown)
    } else {
        Outcome::fail("solutions", format!("missing [{}], unexpected [{}]", missing.join(", "), extra.join(", ")), shown)
    }
}

/// Small deterministic generator for sample points.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn rational(&mut self) -> RBig {
        let n = (self.next() % 199) as i64 - 99;
        let d = (self.next() % 17) + 1;
        RBig::from_parts(IBig::from(n), UBig::from(d))
    }
}

fn check_algebra(latex: &str, settings: &Settings, expect: &Value, shown: String) -> Outcome {
    let parse = |s: &str| parser::parse_with(s, &settings.parse_options()).map(|f| f.expression);
    let input = match parse(latex) {
        Ok(n) => n,
        Err(e) => return Outcome::error(e.to_string()),
    };
    let config = PipelineConfig { style: settings.output_style(), ..PipelineConfig::default() };
    let simplified = match simplify_node_with(&input, config) {
        Ok(s) => s,
        Err(e) => return Outcome::error(e.to_string()),
    };
    let actual = match parse(&shown) {
        Ok(n) => n,
        Err(e) => return Outcome::fail("result-unparsable", e.to_string(), shown),
    };
    let Some(expected_tex) = expect["simplified"].as_str() else { return generator("no simplified form") };
    let expected = match parse(expected_tex) {
        Ok(n) => n,
        Err(e) => return generator(&format!("expected form does not parse: {e}")),
    };

    // Expected restrictions.
    let mut want_values: BTreeSet<(String, RBig)> = BTreeSet::new();
    if let Some(m) = expect["restrictions"].as_object() {
        for (var, vals) in m {
            for v in vals.as_array().into_iter().flatten() {
                if let Some(r) = v.as_str().and_then(parse_rational) {
                    want_values.insert((var.clone(), r));
                }
            }
        }
    }
    let mut want_polys = BTreeSet::new();
    for p in expect["poly_restrictions"].as_array().into_iter().flatten() {
        let Some(tex) = p.as_str() else { continue };
        match parse(tex).ok().and_then(|n| Poly::from_node(&n).ok()) {
            Some(poly) => {
                want_polys.insert(poly.to_monic().to_plain_string());
            }
            None => return generator(&format!("poly restriction {tex} does not parse")),
        }
    }

    // Equivalence at sample points avoiding the expected restrictions.
    let mut variables = collect_variables(&input);
    variables.extend(collect_variables(&expected));
    variables.extend(collect_variables(&actual));
    let mut rng = Lcg(latex.bytes().fold(1469598103934665603u64, |h, b| (h ^ b as u64).wrapping_mul(1099511628211)));
    let mut checked = 0;
    let mut generator_mismatch = None;
    let mut mismatch = None;
    for _ in 0..40 {
        if checked >= 8 {
            break;
        }
        let env: BTreeMap<String, RBig> = variables.iter().map(|v| (v.clone(), rng.rational())).collect();
        if want_values.iter().any(|(v, r)| env.get(v) == Some(r)) {
            continue;
        }
        let (Some(i), Some(e)) = (eval_rational(&input, &env), eval_rational(&expected, &env)) else { continue };
        if i != e && generator_mismatch.is_none() {
            generator_mismatch = Some(format!("input {} vs expected {} at {}", show(&i), show(&e), env_text(&env)));
        }
        match eval_rational(&actual, &env) {
            Some(a) if a == e => {}
            Some(a) => {
                if mismatch.is_none() {
                    mismatch = Some(format!("expected {} got {} at {}", show(&e), show(&a), env_text(&env)));
                }
            }
            None => {
                if mismatch.is_none() {
                    mismatch = Some(format!("result undefined at {}", env_text(&env)));
                }
            }
        }
        checked += 1;
    }
    let actual_restr = format_restrictions_plain(&simplified.domain_restrictions);
    let shown_full = if actual_restr.is_empty() { shown.clone() } else { format!("{shown}  [{actual_restr}]") };
    if let Some(g) = generator_mismatch {
        return Outcome { status: "GENERATOR", reason: String::new(), detail: g, actual: shown_full, note: String::new() };
    }
    if let Some(m) = mismatch {
        return Outcome::fail("wrong-value", m, shown_full);
    }
    if checked == 0 {
        return generator("no sample point could be evaluated");
    }

    // Fully reduced: numerator and denominator share no factor.
    if let Some((num, den)) = split_fraction(strip_negation(&actual)) {
        if let (Ok(n), Ok(d)) = (Poly::from_node(num), Poly::from_node(den)) {
            let g = n.gcd(&d);
            if !g.is_constant() {
                let divides = |p: &Poly| p.div_rem(&g).map(|(_, r)| r.is_zero()).unwrap_or(false);
                if divides(&n) && divides(&d) {
                    return Outcome::fail("not-reduced", format!("common factor {}", g.to_plain_string()), shown_full);
                }
            }
        }
    }

    // Restrictions.
    let mut got_values = BTreeSet::new();
    let mut got_polys = BTreeSet::new();
    for r in &simplified.domain_restrictions {
        match r {
            DomainRestriction::VariableNotEqual { variable, value } => {
                got_values.insert((variable.clone(), value.clone()));
            }
            DomainRestriction::PolyNotZero(p) => {
                got_polys.insert(p.to_monic().to_plain_string());
            }
            // The cases state excluded values only; `u \geq 0` and `u > 0` come from identities of
            // roots and logarithms, which the generated algebra cases do not contain.
            DomainRestriction::PolyNonNegative(_) | DomainRestriction::PolyPositive(_) => {}
        }
    }
    if got_values != want_values || got_polys != want_polys {
        let missing: Vec<String> = want_values
            .difference(&got_values)
            .map(|(v, r)| format!("{v} != {r}"))
            .chain(want_polys.difference(&got_polys).map(|p| format!("{p} != 0")))
            .collect();
        let extra: Vec<String> = got_values
            .difference(&want_values)
            .map(|(v, r)| format!("{v} != {r}"))
            .chain(got_polys.difference(&want_polys).map(|p| format!("{p} != 0")))
            .collect();
        return Outcome::fail(
            "restrictions",
            format!("missing [{}], unexpected [{}]", missing.join(", "), extra.join(", ")),
            shown_full,
        );
    }
    Outcome::pass(shown_full)
}

fn strip_negation(node: &Node) -> &Node {
    match node {
        Node::Negation { operand, .. } => strip_negation(operand),
        Node::Group { inner, .. } => strip_negation(inner),
        other => other,
    }
}

fn env_text(env: &BTreeMap<String, RBig>) -> String {
    env.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join(", ")
}

// --- driver -----------------------------------------------------------------------------------

struct Record {
    case: Arc<Value>,
    outcome: Outcome,
    millis: f64,
}

fn load(dir: &Path, filters: &[String]) -> Vec<Arc<Value>> {
    let mut files = Vec::new();
    collect_json(dir, &mut files);
    files.sort();
    let mut cases = Vec::new();
    for f in files {
        let text = fs::read_to_string(&f).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        let doc: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
        for c in doc["cases"].as_array().into_iter().flatten() {
            let id = c["id"].as_str().unwrap_or_default();
            let cat = c["category"].as_str().unwrap_or_default();
            let sub = format!("{cat}/{}", c["subcategory"].as_str().unwrap_or_default());
            if filters.is_empty() || filters.iter().any(|f| id.starts_with(f.as_str()) || cat == f || sub.starts_with(f.as_str())) {
                cases.push(Arc::new(c.clone()));
            }
        }
    }
    cases
}

fn collect_json(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { panic!("cannot read {}", dir.display()) };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_json(&p, out);
        } else if p.extension().is_some_and(|x| x == "json") {
            out.push(p);
        }
    }
}

/// Address-space limit for each worker process in KiB. A runaway evaluation hits this and dies alone
/// instead of growing until the kernel's OOM killer takes the whole desktop session with it.
const WORKER_LIMIT_KIB: u64 = 4 * 1024 * 1024;

/// A child process that evaluates cases one at a time. A thread cannot be killed, so a case that
/// hangs would keep allocating after its timeout; a process can be, and its memory comes back.
struct Worker {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    lines: mpsc::Receiver<String>,
}

impl Worker {
    fn start() -> Worker {
        let exe = std::env::current_exe().expect("current_exe");
        let script = format!("ulimit -v {WORKER_LIMIT_KIB} && exec \"$0\" --worker");
        let mut child = std::process::Command::new("sh")
            .arg("-c")
            .arg(script)
            .arg(exe)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("start worker process");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            use std::io::BufRead;
            for line in std::io::BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        Worker { child, stdin, lines }
    }

    fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn run(&mut self, case: &Value, timeout: Duration) -> (Outcome, f64) {
        let start = Instant::now();
        let sent = writeln!(self.stdin, "{case}").and_then(|_| self.stdin.flush());
        let outcome = match sent.is_ok().then(|| self.lines.recv_timeout(timeout)) {
            Some(Ok(line)) => outcome_from_json(&serde_json::from_str(&line).unwrap_or(Value::Null)),
            Some(Err(mpsc::RecvTimeoutError::Timeout)) => {
                self.restart();
                Outcome {
                    status: "TIMEOUT",
                    reason: String::new(),
                    detail: format!("no result after {timeout:?}"),
                    actual: String::new(),
                    note: String::new(),
                }
            }
            _ => {
                self.restart();
                Outcome::error("worker process died (out of memory or stack overflow)".into())
            }
        };
        (outcome, start.elapsed().as_secs_f64() * 1000.0)
    }

    fn restart(&mut self) {
        self.stop();
        *self = Worker::start();
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop();
    }
}

fn outcome_to_json(o: &Outcome) -> Value {
    json!({"status": o.status, "reason": o.reason, "detail": o.detail, "actual": o.actual, "note": o.note})
}

fn outcome_from_json(v: &Value) -> Outcome {
    let text = |k: &str| v[k].as_str().unwrap_or_default().to_string();
    let status = match v["status"].as_str().unwrap_or_default() {
        "PASS" => "PASS",
        "FAIL" => "FAIL",
        "UNEXPECTED_OK" => "UNEXPECTED_OK",
        "PANIC" => "PANIC",
        "TIMEOUT" => "TIMEOUT",
        "GENERATOR" => "GENERATOR",
        _ => "ERROR",
    };
    Outcome { status, reason: text("reason"), detail: text("detail"), actual: text("actual"), note: text("note") }
}

/// The body of a worker process: one case per input line, one outcome per output line.
fn worker_main() {
    std::panic::set_hook(Box::new(|_| {}));
    let stdin = std::io::stdin();
    let mut line = String::new();
    while stdin.read_line(&mut line).map(|n| n > 0).unwrap_or(false) {
        let case: Value = serde_json::from_str(&line).expect("case line");
        line.clear();
        let outcome = std::thread::Builder::new()
            .stack_size(256 << 20)
            .spawn(move || {
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_case(&case)));
                match r {
                    Ok(o) => o,
                    Err(panic) => {
                        let msg = panic
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                            .unwrap_or_else(|| "panic".into());
                        Outcome { status: "PANIC", reason: String::new(), detail: msg, actual: String::new(), note: String::new() }
                    }
                }
            })
            .expect("start case thread")
            .join()
            .unwrap_or_else(|_| Outcome::error("case thread failed".into()));
        println!("{}", outcome_to_json(&outcome));
    }
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--worker") {
        return worker_main();
    }
    let opts = options();
    std::panic::set_hook(Box::new(|_| {}));
    let mut cases = load(&opts.cases, &opts.filters);
    if let Some(n) = opts.limit {
        cases.truncate(n);
    }
    let total = cases.len();
    eprintln!("{total} cases, {} jobs", opts.jobs);
    let started = Instant::now();
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let records: Mutex<Vec<Option<Record>>> = Mutex::new((0..total).map(|_| None).collect());
    std::thread::scope(|s| {
        for _ in 0..opts.jobs {
            s.spawn(|| {
                let mut worker = Worker::start();
                loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= total {
                    break;
                }
                let case = cases[i].clone();
                let (outcome, millis) = worker.run(&case, opts.timeout);
                records.lock().unwrap()[i] = Some(Record { case, outcome, millis });
                let d = done.fetch_add(1, Ordering::Relaxed) + 1;
                if d % 5000 == 0 {
                    eprintln!("  {d}/{total} after {:.0} s", started.elapsed().as_secs_f64());
                }
                }
            });
        }
    });
    let records: Vec<Record> = records.into_inner().unwrap().into_iter().map(Option::unwrap).collect();
    let wall = started.elapsed();

    fs::create_dir_all(&opts.out).expect("create output directory");
    let mut jsonl = std::io::BufWriter::new(fs::File::create(opts.out.join("results.jsonl")).expect("results.jsonl"));
    let mut failures = std::io::BufWriter::new(fs::File::create(opts.out.join("failures.jsonl")).expect("failures.jsonl"));
    for r in &records {
        if opts.fail_only && r.outcome.status == "PASS" {
            continue;
        }
        let c = &r.case;
        let line = json!({
            "id": c["id"], "category": c["category"], "subcategory": c["subcategory"], "difficulty": c["difficulty"],
            "gap": c["gap"], "outcome": r.outcome.label(), "detail": r.outcome.detail, "actual": r.outcome.actual,
            "note": r.outcome.note, "ms": (r.millis * 1000.0).round() / 1000.0, "latex": c["latex"], "expect": c["expect"],
        });
        writeln!(jsonl, "{line}").unwrap();
        if r.outcome.status != "PASS" {
            writeln!(failures, "{line}").unwrap();
        }
    }
    jsonl.flush().unwrap();
    failures.flush().unwrap();

    let summary = summarize(&records, wall, &opts);
    fs::write(opts.out.join("summary.md"), &summary).expect("summary.md");
    let passed = records.iter().filter(|r| r.outcome.status == "PASS").count();
    println!(
        "{passed}/{total} passed ({:.2}%) in {:.0} s; report in {}",
        100.0 * passed as f64 / total.max(1) as f64,
        wall.as_secs_f64(),
        opts.out.join("summary.md").display()
    );
}

// --- report -----------------------------------------------------------------------------------

#[derive(Default)]
struct Tally {
    total: usize,
    pass: usize,
    by_outcome: BTreeMap<String, usize>,
}

impl Tally {
    fn add(&mut self, r: &Record) {
        self.total += 1;
        if r.outcome.status == "PASS" {
            self.pass += 1;
        }
        *self.by_outcome.entry(r.outcome.label()).or_default() += 1;
    }
    fn rate(&self) -> String {
        format!("{:.1}%", 100.0 * self.pass as f64 / self.total.max(1) as f64)
    }
    fn outcomes(&self) -> String {
        self.by_outcome
            .iter()
            .filter(|(k, _)| k.as_str() != "PASS")
            .map(|(k, v)| format!("{k} {v}"))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn md(s: &str) -> String {
    let s = s.replace('|', "\\|").replace('\n', " ");
    if s.chars().count() > 160 { format!("{}…", s.chars().take(160).collect::<String>()) } else { s }
}

fn normalize_message(s: &str) -> String {
    let mut out = String::new();
    let mut last_digit = false;
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            if !last_digit {
                out.push('#');
            }
            last_digit = true;
        } else {
            out.push(ch);
            last_digit = false;
        }
    }
    let cut: String = out.chars().take(110).collect();
    // positions and excerpts make every parse error unique
    cut.split(" (at position").next().unwrap_or_default().to_string()
}

fn summarize(records: &[Record], wall: Duration, opts: &Options) -> String {
    let mut all = Tally::default();
    let mut implemented = Tally::default();
    let mut gaps = Tally::default();
    let mut by_cat: BTreeMap<String, Tally> = BTreeMap::new();
    let mut by_sub: BTreeMap<(String, String), Tally> = BTreeMap::new();
    let mut by_diff: BTreeMap<(String, String), Tally> = BTreeMap::new();
    let mut messages: BTreeMap<String, usize> = BTreeMap::new();
    let mut notes: BTreeMap<String, usize> = BTreeMap::new();
    for r in records {
        let c = &r.case;
        let cat = c["category"].as_str().unwrap_or_default().to_string();
        let sub = c["subcategory"].as_str().unwrap_or_default().to_string();
        let diff = c["difficulty"].as_str().unwrap_or_default().to_string();
        all.add(r);
        if c["gap"].as_bool().unwrap_or(false) { gaps.add(r) } else { implemented.add(r) }
        by_cat.entry(cat.clone()).or_default().add(r);
        by_sub.entry((cat.clone(), sub)).or_default().add(r);
        by_diff.entry((cat, diff)).or_default().add(r);
        if matches!(r.outcome.status, "ERROR" | "PANIC") {
            *messages.entry(format!("{}: {}", r.outcome.status, normalize_message(&r.outcome.detail))).or_default() += 1;
        }
        if !r.outcome.note.is_empty() {
            *notes.entry(normalize_message(&r.outcome.note)).or_default() += 1;
        }
    }

    let mut s = String::new();
    let _ = writeln!(s, "# Engine test run: {} cases\n", all.total);
    let _ = writeln!(
        s,
        "**{} / {} passed ({})**, run in {:.0} s with {} jobs and a {:?} limit per case.\n",
        all.pass, all.total, all.rate(), wall.as_secs_f64(), opts.jobs, opts.timeout
    );
    let _ = writeln!(s, "Generated by `cargo run --release -p quantus-core --example run_cases`; the cases are in `test/cases`, \
        one line per case is in `results.jsonl` (not committed), and the cases that did not pass are in `failures.jsonl`. See `test/README.md` for the pass criteria.\n");
    let _ = writeln!(s, "| Scope | Cases | Pass | Rate | Other outcomes |\n|---|---:|---:|---:|---|");
    for (name, t) in [("Implemented features", &implemented), ("Not implemented (`gap`)", &gaps), ("All", &all)] {
        let _ = writeln!(s, "| {name} | {} | {} | {} | {} |", t.total, t.pass, t.rate(), t.outcomes());
    }

    let _ = writeln!(s, "\n## By category\n\n| Category | Cases | Pass | Rate | Other outcomes |\n|---|---:|---:|---:|---|");
    for (cat, t) in &by_cat {
        let _ = writeln!(s, "| {cat} | {} | {} | {} | {} |", t.total, t.pass, t.rate(), t.outcomes());
    }

    let _ = writeln!(s, "\n## By difficulty\n\n| Category | easy | medium | hard | very-hard |\n|---|---:|---:|---:|---:|");
    for cat in by_cat.keys() {
        let cell = |d: &str| by_diff.get(&(cat.clone(), d.to_string())).map(|t| format!("{} of {}", t.rate(), t.total)).unwrap_or_default();
        let _ = writeln!(s, "| {cat} | {} | {} | {} | {} |", cell("easy"), cell("medium"), cell("hard"), cell("very-hard"));
    }

    let _ = writeln!(s, "\n## By subcategory\n\n| Category | Subcategory | Cases | Pass | Rate | Other outcomes |\n|---|---|---:|---:|---:|---|");
    for ((cat, sub), t) in &by_sub {
        let _ = writeln!(s, "| {cat} | {sub} | {} | {} | {} | {} |", t.total, t.pass, t.rate(), t.outcomes());
    }

    let mut top: Vec<_> = messages.into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let _ = writeln!(s, "\n## Most common engine errors and panics\n\nDigits are replaced by `#`.\n\n| Cases | Message |\n|---:|---|");
    for (m, n) in top.iter().take(25) {
        let _ = writeln!(s, "| {n} | {} |", md(m));
    }

    if !notes.is_empty() {
        let _ = writeln!(s, "\n## Notes on passing cases\n\n| Cases | Note |\n|---:|---|");
        let mut notes: Vec<_> = notes.into_iter().collect();
        notes.sort_by(|a, b| b.1.cmp(&a.1));
        for (m, n) in notes.iter().take(15) {
            let _ = writeln!(s, "| {n} | {} |", md(m));
        }
    }

    let _ = writeln!(s, "\n## Example failures\n\nUp to five per subcategory.\n");
    for (cat, sub) in by_sub.keys() {
        let failing: Vec<&Record> = records
            .iter()
            .filter(|r| r.outcome.status != "PASS" && r.case["category"] == cat.as_str() && r.case["subcategory"] == sub.as_str())
            .take(5)
            .collect();
        if failing.is_empty() {
            continue;
        }
        let _ = writeln!(s, "### {cat} / {sub}\n\n| Id | Input | Outcome | Detail | Actual |\n|---|---|---|---|---|");
        for r in failing {
            let _ = writeln!(
                s,
                "| {} | `{}` | {} | {} | {} |",
                r.case["id"].as_str().unwrap_or_default(),
                md(r.case["latex"].as_str().unwrap_or_default()),
                r.outcome.label(),
                md(&r.outcome.detail),
                if r.outcome.actual.is_empty() { String::new() } else { format!("`{}`", md(&r.outcome.actual)) },
            );
        }
        s.push('\n');
    }

    let mut slow: Vec<&Record> = records.iter().collect();
    slow.sort_by(|a, b| b.millis.total_cmp(&a.millis));
    let _ = writeln!(s, "## Slowest cases\n\n| Id | ms | Outcome | Input |\n|---|---:|---|---|");
    for r in slow.iter().take(20) {
        let _ = writeln!(
            s,
            "| {} | {:.0} | {} | `{}` |",
            r.case["id"].as_str().unwrap_or_default(),
            r.millis,
            r.outcome.label(),
            md(r.case["latex"].as_str().unwrap_or_default())
        );
    }
    s
}
