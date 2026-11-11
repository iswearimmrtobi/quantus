//! The card's apps (apps.md §2): which files are listed, a newer API, an id a built-in app has.

use std::path::PathBuf;

use quantus_apps::Apps;
use quantus_ui::applet::AppKind;
use quantus_ui::text::Lang;

fn card() -> Apps {
    Apps::new(Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/card")))
}

#[test]
fn good_files_are_listed_and_bad_ones_left_out() {
    let list = card().list(Lang::De);
    let card: Vec<(&str, &str)> = list.iter().filter(|app| !app.built_in).map(|app| (app.id.as_str(), app.name.as_str())).collect();
    // Sorted by name; no file without a name, with another ending or a space in its name.
    assert_eq!(card, [("card:2048", "2048 von Ada"), ("morgen", "Morgen"), ("schlange", "Schlange"), ("teiler", "Teiler")]);
    let morgen = list.iter().find(|app| app.id == "morgen").unwrap();
    assert!(!morgen.usable);
    let schlange = list.iter().find(|app| app.id == "schlange").unwrap();
    assert_eq!(schlange.kind, AppKind::Game);
    // The built-in 2048 is there as well.
    assert!(list.iter().any(|app| app.id == "2048" && app.built_in));
}

#[test]
fn the_cards_app_opens_beside_the_built_in_one() {
    let apps = card();
    assert!(apps.source("card:2048").unwrap().contains("Ada"));
    assert!(!apps.source("2048").unwrap().contains("Ada"));
    assert!(apps.source("teiler").is_some());
    assert!(apps.open("teiler").is_ok());
    assert!(apps.source("../card/teiler").is_none());
    assert!(apps.source("card:../x").is_none());
}

#[test]
fn a_file_over_the_limit_is_left_out() {
    let folder = std::env::temp_dir().join(format!("quantus-apps-{}-big", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    let big = format!("-- name: Gross\nfunction draw() end\n-- {}\n", "x".repeat(quantus_apps::api::MAX_SOURCE));
    std::fs::write(folder.join("gross.lua"), big).unwrap();
    std::fs::write(folder.join("klein.lua"), "-- name: Klein\nfunction draw() end\n").unwrap();
    let apps = Apps::new(Some(folder));
    let names: Vec<String> = apps.list(Lang::De).into_iter().filter(|app| !app.built_in).map(|app| app.name).collect();
    assert_eq!(names, ["Klein"]);
    assert!(apps.source("gross").is_none());
}

#[test]
fn the_data_of_a_card_app_beside_a_built_in_one_are_its_own() {
    let folder = std::env::temp_dir().join(format!("quantus-apps-{}-twins", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let apps = Apps::new(Some(folder.clone()));
    apps.store_data("2048", "{\"a\":1}").unwrap();
    apps.store_data("card:2048", "{\"b\":2}").unwrap();
    assert_eq!(apps.data("2048").as_deref(), Some("{\"a\":1}"));
    assert_eq!(apps.data("card:2048").as_deref(), Some("{\"b\":2}"));
}
