//! Lists the files of `locales/`, English first, as `include_str!`s: the macros read them from
//! there, and cargo rebuilds when one changes or a language is added.

use std::path::Path;

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../locales").canonicalize().expect("the locales folder");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut codes: Vec<String> = std::fs::read_dir(&dir)
        .expect("the locales folder")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "json").then(|| path.file_stem()?.to_str().map(str::to_string))?
        })
        .collect();
    codes.sort_by_key(|code| (code != "en", code.clone()));
    let mut out = String::from("/// Each language's code and its file, English first.\nconst LOCALES: &[(&str, &str)] = &[\n");
    for code in codes {
        let file = dir.join(format!("{code}.json"));
        println!("cargo:rerun-if-changed={}", file.display());
        out.push_str(&format!("    ({code:?}, include_str!({:?})),\n", file.display().to_string()));
    }
    out.push_str("];\n");
    std::fs::write(Path::new(&std::env::var("OUT_DIR").unwrap()).join("locales.rs"), out).unwrap();
}
