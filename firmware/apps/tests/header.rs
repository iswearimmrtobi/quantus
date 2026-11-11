//! The header of an app's file (apps.md §2).

use quantus_apps::header::parse;
use quantus_apps::info;
use quantus_ui::applet::AppKind;
use quantus_ui::text::Lang;

const FULL: &str = "-- name: Slide
-- name.de: Schieben
-- kind: game
-- api: 1
-- author: Ada
-- author: Grace
-- version: 1.0
-- about: Slide the tiles.
-- about.de: Schiebe die Kacheln.
-- license: MIT

function draw() end
";

#[test]
fn every_line_is_read() {
    let header = parse(FULL).expect("a header");
    assert_eq!(header.name, "Slide");
    assert_eq!(header.kind, AppKind::Game);
    assert_eq!(header.api, 1);
    assert_eq!(header.authors, ["Ada", "Grace"]);
    assert_eq!(header.version.as_deref(), Some("1.0"));
    assert_eq!(header.license.as_deref(), Some("MIT"));
    assert_eq!(header.name(Lang::De), "Schieben");
    assert_eq!(header.name(Lang::En), "Slide");
    assert_eq!(header.about(Lang::De), Some("Schiebe die Kacheln."));
    assert_eq!(header.about(Lang::En), Some("Slide the tiles."));
}

#[test]
fn a_tile_is_asked_for_with_yes() {
    assert!(!parse(FULL).expect("a header").tile);
    for yes in ["yes", "Yes", "true", "1"] {
        assert!(parse(&format!("-- name: T\n-- tile: {yes}\n")).expect("a header").tile, "{yes}");
    }
    assert!(!parse("-- name: T\n-- tile: no\n").expect("a header").tile);
    assert!(info("t", "-- name: T\n-- tile: yes\n", false, Lang::En).expect("an app").tile);
}

#[test]
fn no_name_no_app() {
    assert_eq!(parse("-- kind: game\nfunction draw() end"), None);
    assert_eq!(parse(""), None);
    assert_eq!(parse("-- name:\n"), None);
}

#[test]
fn a_bom_and_crlf_are_read() {
    let header = parse("\u{feff}-- name: Zähler\r\n-- kind: tool\r\n\r\nfunction draw() end\r\n").expect("a header");
    assert_eq!(header.name, "Zähler");
    assert_eq!(header.kind, AppKind::Tool);
}

#[test]
fn a_header_after_code_is_ignored() {
    let header = parse("-- name: Early\nlocal x = 1\n-- author: Late\n").expect("a header");
    assert!(header.authors.is_empty());
}

#[test]
fn values_are_cut_and_the_api_is_checked() {
    let long = "x".repeat(200);
    let header = parse(&format!("-- name: {long}\n")).expect("a header");
    assert_eq!(header.name.chars().count(), 80);
    // Without `api` the first; a newer one is listed as not usable.
    let app = info("old", "-- name: Old\n", true, Lang::De).expect("an app");
    assert!(app.usable && app.api == 1 && app.built_in && app.kind == AppKind::Tool);
    let app = info("new", "-- name: New\n-- api: 7\n", false, Lang::De).expect("an app");
    assert!(!app.usable);
    let app = info("bad", "-- name: Bad\n-- api: one\n", false, Lang::De).expect("an app");
    assert!(!app.usable);
}
