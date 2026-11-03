use quantus_core::api;
use quantus_core::analysis::interface::AnalysisRequest;
use quantus_core::settings::{Settings, Mode, Language};
fn main() {
    let f = std::env::args().nth(1).unwrap();
    let s = Settings { mode: Mode::Analysis, language: Language::De, ..Settings::default() };
    let t = std::time::Instant::now();
    let r = api::analyze(&s, &AnalysisRequest { latex: f, variable: None, precision: Some(6) }).unwrap();
    println!("{} ms; {}", t.elapsed().as_millis(), r.function);
    for sec in &r.sections {
        println!("== {} [{}] {} {:?}", sec.id, sec.title, sec.status, sec.message);
        for st in &sec.steps { println!("  - {:?} | {:?} | {}", st.explanation, st.rule, st.latex); }
        println!("  => {:?}", sec.result);
    }
    for p in &r.points { println!("P {} {} {} {} {}", p.kind, p.name, p.x, p.y, p.latex); }
    for a in &r.asymptotes { println!("A {} {:?} {}", a.kind, a.side, a.latex); }
    println!("plot {} {} {} {} segs={} pts={}", r.plot.x_min, r.plot.x_max, r.plot.y_min, r.plot.y_max, r.plot.segments.len(), r.plot.segments.iter().map(|s| s.len()).sum::<usize>());
}
