//! The Kirchhoff mode through the API: circuits worked out by hand, and the errors a drawing can
//! run into.

use quantus_core::api::{kirchhoff, KirchhoffRequest, KirchhoffResponse};
use quantus_core::settings::{Language, Settings};
use serde_json::{json, Value};

/// A part by ID, the kind from its first letter, with its value as typed and its terminals `a`
/// and `b` in grid steps.
fn part(id: &str, value: &str, a: (i64, i64), b: (i64, i64)) -> Value {
    let (kind, key) = match &id[..1] {
        "V" => ("voltageSource", "u"),
        "I" => ("currentSource", "i"),
        "W" => ("wire", ""),
        "P" => ("potentiometer", "rTotal"),
        _ => ("resistor", "r"),
    };
    let mut part = json!({ "id": id, "kind": kind, "pa": { "x": a.0, "y": a.1 }, "pb": { "x": b.0, "y": b.1 } });
    if !key.is_empty() && !value.is_empty() {
        part[key] = json!(value);
    }
    part
}

fn request(parts: Vec<Value>) -> KirchhoffRequest {
    serde_json::from_value(json!({ "elements": parts })).unwrap()
}

fn solve(parts: Vec<Value>) -> KirchhoffResponse {
    kirchhoff(&Settings { precision: 6, ..Settings::default() }, &request(parts)).unwrap()
}

fn error(parts: Vec<Value>) -> String {
    kirchhoff(&Settings::default(), &request(parts)).unwrap_err().error
}

/// The result of a part: `voltage`, `current` or `power`.
fn shown(response: &KirchhoffResponse, id: &str, what: &str) -> String {
    let element = response.elements.iter().find(|e| e.id == id).unwrap_or_else(|| panic!("no {id}"));
    let value = match what {
        "voltage" => &element.voltage,
        "current" => &element.current,
        _ => &element.power,
    };
    value.result.clone().unwrap()
}

/// 10 V on the left and 5 V on the right, both `+` at the top; R1 = 1 kΩ and R2 = 2 kΩ along the
/// top wire, R3 = 2 kΩ from its middle down to the bottom wire.
fn two_sources() -> Vec<Value> {
    vec![
        part("V1", "10V", (0, 2), (0, 6)),
        part("V2", "5V", (8, 2), (8, 6)),
        part("R1", "1k\\Omega", (0, 0), (4, 0)),
        part("R2", "2k\\Omega", (4, 0), (8, 0)),
        part("R3", "2k\\Omega", (4, 2), (4, 6)),
        part("W1", "", (0, 0), (0, 2)),
        part("W2", "", (0, 6), (0, 8)),
        part("W3", "", (4, 0), (4, 2)),
        part("W4", "", (4, 6), (4, 8)),
        part("W5", "", (8, 0), (8, 2)),
        part("W6", "", (8, 6), (8, 8)),
        part("W7", "", (0, 8), (8, 8)),
    ]
}

/// The bridge of the equations tests: R1, R2 on top, R3, R4 below, R5 across the middle, V1
/// under it with `+` on the left.
fn wheatstone(values: [&str; 6]) -> Vec<Value> {
    let [r1, r2, r3, r4, r5, u] = values;
    vec![
        part("R1", r1, (0, 0), (4, 0)),
        part("R2", r2, (4, 0), (8, 0)),
        part("R3", r3, (0, 8), (4, 8)),
        part("R4", r4, (4, 8), (8, 8)),
        part("R5", r5, (4, 2), (4, 6)),
        part("V1", u, (2, 12), (6, 12)),
        part("W1", "", (4, 0), (4, 2)),
        part("W2", "", (4, 6), (4, 8)),
        part("W3", "", (0, 0), (0, 8)),
        part("W4", "", (0, 8), (0, 12)),
        part("W5", "", (0, 12), (2, 12)),
        part("W6", "", (6, 12), (8, 12)),
        part("W7", "", (8, 12), (8, 8)),
        part("W8", "", (8, 8), (8, 0)),
    ]
}

/// By hand: the middle of the top wire is at φ with (10 − φ)/1k = φ/2k + (φ − 5)/2k, so
/// φ = 6.25 V. I(R1) = 3.75 mA, I(R2) = 0.625 mA towards V2, I(R3) = 3.125 mA.
#[test]
fn two_sources_with_three_resistors() {
    let solved = solve(two_sources());
    assert_eq!(solved.error, None);
    assert_eq!(solved.unknowns, ["I_{1}", "I_{2}", "I_{3}"]);
    assert_eq!(solved.junctions.len(), 2);
    assert_eq!(solved.junctions.iter().filter(|j| j.dropped).count(), 1);
    assert_eq!(solved.loops.len(), 2);
    assert_eq!(solved.matrix.len(), 3);
    assert_eq!(solved.branches.iter().map(|b| b.lead.as_str()).collect::<Vec<_>>(), ["V1", "V2", "R3"]);
    // Every arrow follows its current: I_2 runs into V2's `+`, so no current is negative.
    assert_eq!(
        solved.result,
        [
            "I_{1} = \\frac{3}{800}\\,\\mathrm{A}",
            "I_{2} = \\frac{1}{1600}\\,\\mathrm{A}",
            "I_{3} = \\frac{1}{320}\\,\\mathrm{A}"
        ]
    );
    assert_eq!(shown(&solved, "R1", "current"), "3{,}75\\,\\mathrm{mA}");
    assert_eq!(shown(&solved, "R2", "current"), "625\\,\\mathrm{\\mu A}");
    assert_eq!(shown(&solved, "R3", "voltage"), "6{,}25\\,\\mathrm{V}");
    // A source's I is what it delivers, so V2, which is charged, delivers a negative power.
    assert_eq!(shown(&solved, "V1", "current"), "3{,}75\\,\\mathrm{mA}");
    assert_eq!(shown(&solved, "V1", "power"), "37{,}5\\,\\mathrm{mW}");
    assert_eq!(shown(&solved, "V2", "power"), "-3{,}125\\,\\mathrm{mW}");
    // 37.5 mW − 3.125 mW go into the resistors.
    let balance = solved.power_balance.unwrap();
    assert_eq!(balance.generated, balance.consumed);
    assert_eq!(balance.generated.result.as_deref(), Some("34{,}375\\,\\mathrm{mW}"));
    assert!(solved.notes.iter().any(|n| n.contains("follows from the others")), "{:?}", solved.notes);
    assert!(solved.align_block.starts_with("\\begin{align*}") && solved.align_block.contains("\\begin{cases}"));
}

/// Balanced, R1/R2 = R3/R4: no current through R5, and R1 takes a third of the 10 V.
#[test]
fn a_balanced_wheatstone_bridge() {
    let solved = solve(wheatstone(["1k\\Omega", "2k\\Omega", "1k\\Omega", "2k\\Omega", "500\\Omega", "10V"]));
    assert_eq!(solved.error, None);
    assert_eq!(solved.junctions.len(), 4);
    assert_eq!(solved.loops.len(), 3);
    assert_eq!(solved.unknowns.len(), 6);
    assert_eq!(shown(&solved, "R5", "current"), "0\\,\\mathrm{A}");
    assert_eq!(shown(&solved, "R1", "voltage"), "3{,}33333\\,\\mathrm{V}");
    assert_eq!(shown(&solved, "R4", "current"), "3{,}33333\\,\\mathrm{mA}");
    assert_eq!(shown(&solved, "V1", "current"), "6{,}66667\\,\\mathrm{mA}");
    assert_eq!(solved.power_balance.unwrap().consumed.result.as_deref(), Some("66{,}6667\\,\\mathrm{mW}"));
    // Each loop's members, clockwise: the first window is R1, R5 and R3.
    let first: Vec<&str> = solved.loops[0].members.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(first, ["R1", "R5", "R3"]);
    assert_eq!(solved.loops[0].symbolic, "R_{1} I_{2} + R_{5} I_{6} - R_{3} I_{4} = 0");
}

/// 100 V, R1 in series with R2 ∥ R3, all 1 kΩ; R1 and R2 are drawn turned round, from right to
/// left, so the current runs through them from `b` to `a`.
fn turned_round(reversed: &str) -> Vec<Value> {
    let mut parts = vec![
        part("V1", "100V", (32, 16), (36, 16)),
        part("R1", "1k\\Omega", (32, 11), (28, 11)),
        part("R2", "1k\\Omega", (39, 7), (35, 7)),
        part("R3", "1k\\Omega", (35, 11), (39, 11)),
        part("W1", "", (35, 7), (35, 11)),
        part("W2", "", (35, 11), (32, 11)),
        part("W3", "", (39, 7), (39, 16)),
        part("W4", "", (28, 11), (28, 16)),
        part("W5", "", (32, 16), (28, 16)),
        part("W6", "", (36, 16), (39, 16)),
    ];
    for part in parts.iter_mut().filter(|p| p["id"] == reversed) {
        part["reversed"] = json!(true);
    }
    parts
}

/// The arrows follow the real current whichever way the parts are drawn, so every current and
/// every resistor's U, I and P is positive; a flipped arrow shows its current negative.
#[test]
fn the_arrows_follow_the_current() {
    let solved = solve(turned_round(""));
    assert_eq!(solved.error, None);
    assert!(solved.result.iter().all(|r| !r.contains("= -")), "{:?}", solved.result);
    for id in ["R1", "R2", "R3", "V1"] {
        for what in ["voltage", "current", "power"] {
            assert!(!shown(&solved, id, what).starts_with('-'), "{id} {what}");
        }
    }
    assert_eq!(shown(&solved, "R1", "current"), "66{,}6667\\,\\mathrm{mA}");
    assert_eq!(shown(&solved, "R2", "current"), "33{,}3333\\,\\mathrm{mA}");
    assert_eq!(shown(&solved, "V1", "power"), "6{,}66667\\,\\mathrm{W}");

    let flipped = solve(turned_round("R2"));
    let k = flipped.branches.iter().position(|b| b.lead == "R2").unwrap();
    assert!(flipped.result[k].contains("= -"), "{:?}", flipped.result);
    assert_eq!(shown(&flipped, "R2", "current"), "-33{,}3333\\,\\mathrm{mA}");
    assert_eq!(shown(&flipped, "R2", "power"), "1{,}11111\\,\\mathrm{W}");
}

#[test]
fn the_steps_follow_the_detail_level_and_the_method() {
    let at = |settings: Settings| kirchhoff(&settings, &request(two_sources())).unwrap();
    let brief = at(Settings { step_detail: quantus_core::settings::StepDetail::Brief, ..Settings::default() });
    let maximal = at(Settings { step_detail: quantus_core::settings::StepDetail::Maximal, ..Settings::default() });
    assert!(maximal.steps.len() > brief.steps.len());
    assert_eq!(brief.steps[0].kind, "symbolic");
    assert_eq!(brief.steps[1].kind, "substituted");
    assert_eq!(brief.steps.last().unwrap().kind, "result");
    let jordan = at(Settings { elimination: quantus_core::settings::Elimination::GaussJordan, ..Settings::default() });
    assert_ne!(jordan.steps, at(Settings::default()).steps);
    assert_eq!(jordan.result, at(Settings::default()).result);
}

#[test]
fn names_and_texts_follow_the_language() {
    let german = Settings { language: Language::De, ..Settings::default() };
    let solved = kirchhoff(&german, &request(two_sources())).unwrap();
    assert_eq!(solved.junctions[0].name, "K_{1}");
    assert_eq!(solved.loops[0].name, "M_{1}");
    assert!(solved.notes.iter().any(|n| n.contains("Knotenregel bei K₂")), "{:?}", solved.notes);
    assert!(solved.power_balance.unwrap().latex.contains("P_{\\mathrm{abg}}"));
}

#[test]
fn an_unsupported_part_is_named() {
    let mut parts = two_sources();
    parts.push(part("P1", "1k\\Omega", (12, 0), (16, 0)));
    let message = error(parts);
    assert!(message.contains("P1") && message.contains("Kirchhoff"), "{message}");

    let mut parts = two_sources();
    parts[5]["material"] = json!("copper");
    parts[5]["length"] = json!("1m");
    parts[5]["area"] = json!("1mm^2");
    let message = error(parts);
    assert!(message.contains("W1") && message.contains("ideal wires"), "{message}");
}

#[test]
fn a_missing_value_is_named() {
    let mut parts = two_sources();
    parts[3] = part("R2", "", (4, 0), (8, 0));
    assert_eq!(error(parts), "The value R of R2 is missing.");
    let mut parts = two_sources();
    parts[0] = part("V1", "", (0, 2), (0, 6));
    assert_eq!(error(parts), "The value U of V1 is missing.");
}

#[test]
fn an_open_end_is_named() {
    let mut parts = two_sources();
    parts.push(part("R4", "1\\Omega", (8, 8), (12, 8)));
    let message = error(parts);
    assert!(message.contains("R4") && message.contains("open end"), "{message}");
}

#[test]
fn a_loop_of_sources_alone_has_no_solution_but_keeps_its_steps() {
    // A wire across V1 makes a loop of V1 alone: 0 = 10 V.
    let shorted = solve(vec![
        part("V1", "10V", (0, 0), (0, 4)),
        part("R1", "1\\Omega", (4, 0), (4, 4)),
        part("W1", "", (0, 0), (4, 0)),
        part("W2", "", (0, 4), (4, 4)),
        part("W3", "", (0, 0), (2, 2)),
        part("W4", "", (2, 2), (0, 4)),
    ]);
    let message = shorted.error.clone().unwrap();
    assert!(message.contains("contradict"), "{message}");
    assert!(shorted.elements.is_empty() && shorted.power_balance.is_none());
    assert!(shorted.steps.len() > 2);
    assert_eq!(shorted.steps.last().unwrap().latex, "\\text{no solution}");

    // Two equal sources side by side: the current between them is open.
    let parallel = solve(vec![
        part("V1", "10V", (0, 0), (0, 4)),
        part("V2", "10V", (4, 0), (4, 4)),
        part("W1", "", (0, 0), (4, 0)),
        part("W2", "", (0, 4), (4, 4)),
    ]);
    assert!(parallel.error.unwrap().contains("open"));

    // Two current sources in series set the same current twice.
    let message = error(vec![
        part("I1", "1A", (0, 0), (0, 4)),
        part("I2", "2A", (4, 4), (4, 0)),
        part("W1", "", (0, 4), (4, 4)),
        part("W2", "", (4, 0), (0, 0)),
    ]);
    assert!(message.contains("I1") && message.contains("I2"), "{message}");
}

#[test]
fn a_drawing_without_parts_is_an_error() {
    assert!(error(vec![]).contains("no parts"));
    assert!(error(vec![part("W1", "", (0, 0), (4, 0))]).contains("no parts"));
}

// The same circuits through the electronics solver (modified nodal analysis) give exactly the same
// voltages, currents and powers; the reference directions change none of them.

use quantus_core::api::{solve_circuit, CircuitRequest};
use quantus_core::electro::interface::{ElementResponse, ValueResponse};

/// A part's terminals `a` and `b`, in grid steps.
type Ends = ((i64, i64), (i64, i64));

/// Where an editor element's terminals sit, in grid steps: a part is four steps long and turned by
/// quarter turns, as `terminals` in `desktop/src/circuit.ts` places it.
fn terminals(element: &Value) -> Ends {
    let at = |key: &str| element[key].as_i64().unwrap_or(0);
    let (x, y) = (at("x"), at("y"));
    if element["kind"] == "wire" {
        let end = |key: &str, or: i64| element[key].as_i64().unwrap_or(or);
        return ((x / 20, y / 20), (end("x2", x) / 20, end("y2", y) / 20));
    }
    let (dx, dy) = match at("rotation").rem_euclid(4) {
        1 => (0, -40),
        2 => (40, 0),
        3 => (0, 40),
        _ => (-40, 0),
    };
    (((x + dx) / 20, (y + dy) / 20), ((x - dx) / 20, (y - dy) / 20))
}

/// An electronics request, with node numbers, as the Kirchhoff mode's: the same parts with their
/// terminal points and the given `reversed` flags.
fn as_kirchhoff(circuit: &Value, points: &[Ends], reversed: &[bool]) -> KirchhoffRequest {
    let elements: Vec<Value> = circuit["elements"]
        .as_array()
        .unwrap()
        .iter()
        .zip(points)
        .zip(reversed)
        .map(|((element, &(a, b)), &reversed)| {
            let mut element = element.clone();
            element["pa"] = json!({ "x": a.0, "y": a.1 });
            element["pb"] = json!({ "x": b.0, "y": b.1 });
            element["reversed"] = json!(reversed);
            element
        })
        .collect();
    serde_json::from_value(json!({ "elements": elements })).unwrap()
}

/// The same value with the other sign; zero stays as it is.
fn negated(value: &ValueResponse) -> ValueResponse {
    let flip = |text: &String| match text.strip_prefix('-') {
        Some(rest) => rest.to_string(),
        None if text.starts_with('0') => text.clone(),
        None => format!("-{text}"),
    };
    ValueResponse { result: value.result.as_ref().map(flip), value: value.value.as_ref().map(flip), ..value.clone() }
}

/// A part as the electronics mode gives it, turned to the Kirchhoff mode's arrows. A resistor
/// comes along its real current there, as its arrow does unless the branch is `flagged` or follows
/// a current source that drives against it; a source comes from `a` to `b` and is turned to
/// delivering.
fn along_arrows(response: &KirchhoffResponse, element: &ElementResponse, flagged: &[String]) -> ElementResponse {
    let branch = response.branches.iter().find(|b| b.members.iter().any(|m| m.id == element.id && !m.internal)).unwrap();
    let mut turned = element.clone();
    match &element.id[..1] {
        "V" => turned.current = negated(&element.current),
        "I" => turned.voltage = negated(&element.voltage),
        _ => {
            let against = if branch.current.contains("I_{q") {
                let solved = response.elements.iter().find(|e| e.id == element.id).unwrap();
                solved.current.value.as_ref().is_some_and(|v| v.starts_with('-'))
            } else {
                flagged.contains(&branch.lead)
            };
            if against {
                turned.voltage = negated(&element.voltage);
                turned.current = negated(&element.current);
            }
            return turned;
        }
    }
    turned.power = negated(&element.power);
    turned
}

/// Every part's U, I, P and R from the Kirchhoff mode equals the electronics mode's, turned to its
/// arrows, for every choice of reference directions.
fn assert_same_as_electronics(name: &str, circuit: &Value, points: &[Ends]) {
    let settings = Settings::default();
    let mut electronic = circuit.clone();
    electronic["elements"].as_array_mut().unwrap().retain(|e| e["kind"] != "ground");
    let request: CircuitRequest = serde_json::from_value(electronic).unwrap();
    let expected = solve_circuit(&settings, &request).unwrap_or_else(|e| panic!("{name}: {e}"));

    let n = points.len();
    let mut seed = n as u64;
    for round in 0..4 {
        // All forward, then pseudo-random flags.
        let reversed: Vec<bool> = (0..n).map(|_| round > 0 && next(&mut seed).is_multiple_of(2)).collect();
        let request = as_kirchhoff(circuit, points, &reversed);
        let flagged: Vec<String> = request.elements.iter().filter(|e| e.reversed).map(|e| e.element.id.clone()).collect();
        let solved = kirchhoff(&settings, &request).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(solved.error, None, "{name}");
        assert!(!solved.elements.is_empty(), "{name}");
        for element in &solved.elements {
            let reference = expected.elements.iter().find(|e| e.id == element.id).unwrap();
            assert_eq!(element, &along_arrows(&solved, reference, &flagged), "{name}, {}, reversed {reversed:?}", element.id);
        }
        let balance = solved.power_balance.unwrap();
        assert_eq!(balance.generated, balance.consumed, "{name}");
    }
}

/// A linear congruential generator, so the generated circuits are the same on every run.
fn next(seed: &mut u64) -> u64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    *seed >> 33
}

#[test]
fn the_test_circuits_match_the_electronics_mode() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/circuits");
    for name in ["divider", "parallel", "precision", "current-source", "two-sources", "wheatstone"] {
        let text = std::fs::read_to_string(directory.join(format!("{name}.json"))).unwrap();
        let circuit: Value = serde_json::from_str(&text).unwrap();
        let points: Vec<_> = circuit["elements"].as_array().unwrap().iter().map(terminals).collect();
        assert_same_as_electronics(name, &circuit, &points);
    }
}

/// A ladder of `columns` columns: a part down each column from the top rail to the bottom wire,
/// and a part along the top between neighbouring columns. Column 0 holds a voltage source, the
/// others resistors or current sources; along the top run resistors and now and then a voltage
/// source. Parts point either way, some sources with an internal resistance or conductance.
fn ladder(columns: i64, seed: &mut u64) -> (Value, Vec<Ends>) {
    let mut elements = Vec::new();
    let mut points = Vec::new();
    let mut wires = 0;
    let mut wire = |a: (i64, i64), b: (i64, i64), elements: &mut Vec<Value>, points: &mut Vec<_>| {
        wires += 1;
        elements.push(json!({ "id": format!("W{wires}"), "kind": "wire" }));
        points.push((a, b));
    };
    // Either way round.
    let turned = |a: (i64, i64), b: (i64, i64), seed: &mut u64| if next(seed).is_multiple_of(2) { (b, a) } else { (a, b) };
    for k in 0..columns {
        let x = 4 * k;
        let down = match (k, next(seed) % 4) {
            (0, choice) => {
                let mut source = json!({ "id": "V1", "kind": "voltageSource", "u": format!("{}V", 1 + next(seed) % 24) });
                if choice == 0 {
                    source["internalResistance"] = json!(format!("{}\\Omega", 1 + next(seed) % 9));
                }
                source
            }
            (_, 0) => {
                let mut source = json!({ "id": format!("I{k}"), "kind": "currentSource", "i": format!("{}A", 1 + next(seed) % 5) });
                if next(seed).is_multiple_of(2) {
                    source["internalConductance"] = json!(format!("{}S", 1 + next(seed) % 4));
                }
                source
            }
            _ => json!({ "id": format!("R{k}"), "kind": "resistor", "r": format!("{}\\Omega", 1 + next(seed) % 1000) }),
        };
        elements.push(down);
        points.push(turned((x, 2), (x, 6), seed));
        wire((x, 0), (x, 2), &mut elements, &mut points);
        wire((x, 6), (x, 8), &mut elements, &mut points);
        if k + 1 < columns {
            let along = if next(seed).is_multiple_of(5) {
                json!({ "id": format!("V{}", k + 2), "kind": "voltageSource", "u": format!("{}V", 1 + next(seed) % 12) })
            } else {
                json!({ "id": format!("R{}", 100 + k), "kind": "resistor", "r": format!("{}\\Omega", 1 + next(seed) % 1000) })
            };
            elements.push(along);
            points.push(turned((x, 0), (x + 4, 0), seed));
            wire((x, 8), (x + 4, 8), &mut elements, &mut points);
        }
    }
    // A node per grid point, as the editor numbers them; the bottom left corner is ground.
    let mut nodes: Vec<(i64, i64)> = vec![(0, 8)];
    for (element, &(a, b)) in elements.iter_mut().zip(&points) {
        for (terminal, point) in [("a", a), ("b", b)] {
            let node = nodes.iter().position(|&p| p == point).unwrap_or_else(|| {
                nodes.push(point);
                nodes.len() - 1
            });
            element[terminal] = json!(node);
        }
    }
    (json!({ "nodes": (0..nodes.len()).collect::<Vec<_>>(), "elements": elements }), points)
}

#[test]
fn generated_ladders_match_the_electronics_mode() {
    let mut seed = 7;
    let mut seen = std::collections::BTreeSet::new();
    for columns in 2..=7 {
        for round in 0..6 {
            let (circuit, points) = ladder(columns, &mut seed);
            for element in circuit["elements"].as_array().unwrap() {
                seen.extend(["internalResistance", "internalConductance"].into_iter().filter(|key| element.get(key).is_some()).map(String::from));
                if element["id"] != "V1" && element["kind"] == "voltageSource" || element["kind"] == "currentSource" {
                    seen.insert(element["kind"].as_str().unwrap().to_string());
                }
            }
            assert_same_as_electronics(&format!("ladder {columns}/{round}"), &circuit, &points);
        }
    }
    assert_eq!(seen.len(), 4, "{seen:?}");
}

#[test]
fn a_flipped_branch_negates_its_current() {
    let (circuit, points) = ladder(4, &mut 3);
    let n = points.len();
    let settings = Settings::default();
    let forward = kirchhoff(&settings, &as_kirchhoff(&circuit, &points, &vec![false; n])).unwrap();
    for (k, branch) in forward.branches.iter().enumerate() {
        // A current source's branch has its voltage as the unknown, which no direction changes.
        if !branch.current.starts_with("I_{") || branch.current.starts_with("I_{q") {
            continue;
        }
        let lead = circuit["elements"].as_array().unwrap().iter().position(|e| e["id"] == branch.lead.as_str()).unwrap();
        let mut reversed = vec![false; n];
        reversed[lead] = true;
        let flipped = kirchhoff(&settings, &as_kirchhoff(&circuit, &points, &reversed)).unwrap();
        for (j, (before, after)) in forward.result.iter().zip(&flipped.result).enumerate() {
            if j != k {
                assert_eq!(before, after);
                continue;
            }
            let negated = match before.split_once("= -") {
                Some((name, size)) => format!("{name}= {size}"),
                None => before.replacen("= ", "= -", 1),
            };
            let zero = before.ends_with("= 0\\,\\mathrm{A}");
            assert_eq!(after, if zero { before } else { &negated }, "{}", branch.lead);
        }
        // The flipped branch's resistors show U and I against their arrows now, the same power.
        for (before, after) in forward.elements.iter().zip(&flipped.elements) {
            let resistor = before.resistance.is_some() && forward.branches[k].members.iter().any(|m| m.id == before.id);
            let expected = if resistor {
                ElementResponse { voltage: negated(&before.voltage), current: negated(&before.current), ..before.clone() }
            } else {
                before.clone()
            };
            assert_eq!(after, &expected, "{}", branch.lead);
        }
    }
}

/// The `kirchhoff-*.json` files in `test/circuits/`, for trying the mode by hand, give the
/// values `test/circuits/README.md` documents.
#[test]
fn the_kirchhoff_test_circuits_solve_to_their_documented_values() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/circuits");
    let load = |name: &str| {
        let text = std::fs::read_to_string(directory.join(format!("kirchhoff-{name}.json"))).unwrap();
        let circuit: Value = serde_json::from_str(&text).unwrap();
        let points: Vec<_> = circuit["elements"].as_array().unwrap().iter().map(terminals).collect();
        let reversed = vec![false; points.len()];
        kirchhoff(&Settings { precision: 6, ..Settings::default() }, &as_kirchhoff(&circuit, &points, &reversed))
    };

    let source = load("current-source").unwrap();
    assert_eq!(source.error, None);
    assert_eq!(shown(&source, "R1", "current"), r"333{,}333\,\mathrm{mA}");
    assert_eq!(shown(&source, "R2", "voltage"), r"9{,}33333\,\mathrm{V}");
    assert_eq!(shown(&source, "R2", "current"), r"2{,}33333\,\mathrm{A}");
    assert!(source.unknowns.contains(&"U_{q,1}".to_string()), "{:?}", source.unknowns);

    let crossing = load("crossing").unwrap();
    assert!(crossing.notes[0].contains("spanning tree"), "{:?}", crossing.notes);
    assert_eq!(shown(&crossing, "R1", "current"), r"3\,\mathrm{A}");
    assert_eq!(shown(&crossing, "R2", "current"), r"1\,\mathrm{A}");
    assert_eq!(shown(&crossing, "V1", "current"), r"4\,\mathrm{A}");

    let sources = load("source-loop").unwrap();
    assert!(sources.error.unwrap().starts_with("The equations contradict each other"));
    assert!(sources.elements.is_empty() && !sources.steps.is_empty());
}

/// The resistance between A at `a` and B at `b` of a drawing of resistors and wires.
fn between(parts: Vec<Value>, a: (i64, i64), b: (i64, i64), settings: &Settings) -> Result<KirchhoffResponse, String> {
    let request: KirchhoffRequest = serde_json::from_value(json!({
        "elements": parts,
        "terminals": [{ "x": a.0, "y": a.1 }, { "x": b.0, "y": b.1 }],
    }))
    .unwrap();
    kirchhoff(settings, &request).map_err(|e| e.error)
}

fn r_ab(parts: Vec<Value>, a: (i64, i64), b: (i64, i64)) -> String {
    let response = between(parts, a, b, &Settings::default()).unwrap();
    response.equivalent.unwrap().resistance.value.unwrap()
}

#[test]
fn resistors_in_series_and_in_parallel() {
    let series = vec![part("R1", "3\\Omega", (0, 0), (4, 0)), part("R2", "4\\Omega", (4, 0), (8, 0))];
    assert_eq!(r_ab(series, (0, 0), (8, 0)), "7");
    // Two parallel resistors drawn as a rectangle: wires on the left and right.
    let parallel = vec![
        part("R1", "6\\Omega", (0, 0), (4, 0)),
        part("R2", "3\\Omega", (0, 4), (4, 4)),
        part("W1", "", (0, 0), (0, 4)),
        part("W2", "", (4, 0), (4, 4)),
    ];
    assert_eq!(r_ab(parallel, (0, 2), (4, 2)), "2");
}

/// R1 from A to X, R2 and R3 from X to B, side by side.
fn r1_and_two_in_parallel() -> Vec<Value> {
    vec![
        part("R1", "1\\Omega", (0, 0), (4, 0)),
        part("R2", "2\\Omega", (4, 0), (8, 0)),
        part("R3", "3\\Omega", (4, 4), (8, 4)),
        part("W1", "", (4, 0), (4, 4)),
        part("W2", "", (8, 0), (8, 4)),
    ]
}

#[test]
fn a_series_resistor_before_a_parallel_pair() {
    let response = between(r1_and_two_in_parallel(), (0, 0), (8, 0), &Settings::default()).unwrap();
    let equivalent = response.equivalent.unwrap();
    // 1 + 2·3/5 = 11/5.
    assert_eq!(equivalent.latex, "R_{AB} = \\frac{11}{5}\\,\\mathrm{\\Omega }");
    let merges: Vec<(&str, Vec<&str>)> =
        equivalent.merges.iter().map(|m| (m.symbol.as_str(), m.parts.iter().map(String::as_str).collect())).collect();
    assert_eq!(merges, [("R_{23}", vec!["R2", "R3"]), ("R_{123}", vec!["R1", "R2", "R3"])]);
    assert!(response.steps.iter().any(|s| s.latex.contains("\\frac{R_{2} \\cdot R_{3}}{R_{2} + R_{3}}")));
    assert!(response.junctions.is_empty() && response.matrix.is_empty());
}

#[test]
fn a_ladder_reduces_from_its_far_end() {
    // Rungs R2, R4, R6 down to the bottom rail; R1, R3, R5 along the top. All 1 Ω.
    let mut parts = vec![
        part("R1", "1\\Omega", (0, 0), (4, 0)),
        part("R3", "1\\Omega", (4, 0), (8, 0)),
        part("R5", "1\\Omega", (8, 0), (12, 0)),
        part("R2", "1\\Omega", (4, 0), (4, 4)),
        part("R4", "1\\Omega", (8, 0), (8, 4)),
        part("R6", "1\\Omega", (12, 0), (12, 4)),
    ];
    parts.push(part("W1", "", (0, 4), (12, 4)));
    // 1 + 1 ∥ (1 + 1 ∥ 2) = 1 + 1 ∥ 5/3 = 1 + 5/8 = 13/8.
    assert_eq!(r_ab(parts, (0, 0), (0, 4)), "1.625");
}

#[test]
fn parts_without_current_are_left_out_with_a_note() {
    let mut parts = r1_and_two_in_parallel();
    parts.push(part("R4", "9\\Omega", (0, 0), (0, 4)));
    parts.push(part("R5", "9\\Omega", (20, 0), (24, 0)));
    let response = between(parts, (0, 0), (8, 0), &Settings::default()).unwrap();
    assert_eq!(response.equivalent.unwrap().resistance.value.as_deref(), Some("2.2"));
    assert!(response.notes.iter().any(|n| n.starts_with("R4 has an open end")), "{:?}", response.notes);
    assert!(response.notes.iter().any(|n| n.starts_with("R5 is not connected")), "{:?}", response.notes);
}

#[test]
fn a_bridge_is_finished_with_the_node_rule() {
    let bridge = |r5: &str| {
        vec![
            part("R1", "1\\Omega", (0, 4), (4, 0)),
            part("R2", "1\\Omega", (0, 4), (4, 8)),
            part("R3", "2\\Omega", (4, 0), (8, 4)),
            part("R4", "2\\Omega", (4, 8), (8, 4)),
            part("R5", r5, (4, 0), (4, 8)),
        ]
    };
    // Balanced: R5 carries no current, so R_AB = 3 ∥ 3.
    let balanced = between(bridge("5\\Omega"), (0, 4), (8, 4), &Settings::default()).unwrap();
    assert_eq!(balanced.equivalent.as_ref().unwrap().resistance.value.as_deref(), Some("1.5"));
    assert_eq!(balanced.unknowns, ["\\varphi_{A}", "\\varphi_{1}", "\\varphi_{2}"]);
    assert!(balanced.notes.iter().any(|n| n.contains("node rule")));
    // Unbalanced: 1, 2 on top, 2, 1 below, 1 across: R_AB = 7/5.
    let mut unbalanced = bridge("1\\Omega");
    unbalanced[1] = part("R2", "2\\Omega", (0, 4), (4, 8));
    unbalanced[3] = part("R4", "1\\Omega", (4, 8), (8, 4));
    assert_eq!(r_ab(unbalanced, (0, 4), (8, 4)), "1.4");
}

#[test]
fn shorts_open_networks_and_sources() {
    let shorted = vec![part("R1", "3\\Omega", (0, 0), (4, 0)), part("W1", "", (0, 0), (4, 0))];
    assert_eq!(r_ab(shorted, (0, 0), (4, 0)), "0");
    let apart = vec![part("R1", "3\\Omega", (0, 0), (4, 0)), part("R2", "3\\Omega", (8, 0), (12, 0))];
    assert!(between(apart, (0, 0), (12, 0), &Settings::default()).unwrap_err().contains("infinite"));
    let off = vec![part("R1", "3\\Omega", (0, 0), (4, 0))];
    assert!(between(off, (0, 0), (9, 9), &Settings::default()).unwrap_err().starts_with("B is not connected"));
    let source = vec![part("R1", "3\\Omega", (0, 0), (4, 0)), part("V1", "5V", (4, 0), (8, 0))];
    assert!(between(source, (0, 0), (8, 0), &Settings::default()).unwrap_err().starts_with("V1 is a source"));
}

#[test]
fn the_reduction_follows_the_detail_level_and_the_language() {
    use quantus_core::settings::StepDetail;
    let at = |step_detail| between(r1_and_two_in_parallel(), (0, 0), (8, 0), &Settings { step_detail, ..Settings::default() }).unwrap();
    let brief = at(StepDetail::Brief);
    assert!(brief.steps[0].latex.starts_with("R_{23} = R_{2} \\parallel R_{3} ="), "{}", brief.steps[0].latex);
    let detailed = at(StepDetail::Detailed);
    assert!(detailed.steps[0].latex.contains("\\frac{2\\,\\mathrm{\\Omega } \\cdot 3\\,\\mathrm{\\Omega }}"), "{}", detailed.steps[0].latex);
    assert_eq!(brief.steps.len(), detailed.steps.len());
    let german = between(r1_and_two_in_parallel(), (0, 0), (8, 0), &Settings { language: Language::De, ..Settings::default() }).unwrap();
    assert_eq!(german.steps[0].explanation.as_deref(), Some("R₂ und R₃ sind parallel: sie liegen zwischen denselben zwei Knoten."));
    assert_eq!(german.steps[1].explanation.as_deref().map(|e| e.starts_with("R₁ und R₂₃ sind in Reihe")), Some(true));
}
