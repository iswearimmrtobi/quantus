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
    // I_1 out of V1's `+`, I_2 out of V2's `+`, against the 0.625 mA that flows into it.
    assert_eq!(
        solved.result,
        [
            "I_{1} = \\frac{3}{800}\\,\\mathrm{A}",
            "I_{2} = -\\frac{1}{1600}\\,\\mathrm{A}",
            "I_{3} = \\frac{1}{320}\\,\\mathrm{A}"
        ]
    );
    assert_eq!(shown(&solved, "R1", "current"), "3{,}75\\,\\mathrm{mA}");
    assert_eq!(shown(&solved, "R2", "current"), "625\\,\\mathrm{\\mu A}");
    assert_eq!(shown(&solved, "R3", "voltage"), "6{,}25\\,\\mathrm{V}");
    // As in the electronics mode: a source that delivers has a negative current and power.
    assert_eq!(shown(&solved, "V1", "current"), "-3{,}75\\,\\mathrm{mA}");
    assert_eq!(shown(&solved, "V1", "power"), "-37{,}5\\,\\mathrm{mW}");
    assert_eq!(shown(&solved, "V2", "power"), "3{,}125\\,\\mathrm{mW}");
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
    assert_eq!(shown(&solved, "V1", "current"), "-6{,}66667\\,\\mathrm{mA}");
    assert_eq!(solved.power_balance.unwrap().consumed.result.as_deref(), Some("66{,}6667\\,\\mathrm{mW}"));
    // Each loop's members, clockwise: the first window is R1, R5 and R3.
    let first: Vec<&str> = solved.loops[0].members.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(first, ["R1", "R5", "R3"]);
    assert_eq!(solved.loops[0].symbolic, "R_{1} I_{2} + R_{5} I_{6} - R_{3} I_{4} = 0");
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
