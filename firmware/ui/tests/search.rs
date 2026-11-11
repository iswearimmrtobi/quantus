//! CATALOG's search: letters typed in it find the engine's entries and the menus' together.

use quantus_ui::app::{App, Mode, Screen, Settings};
use quantus_ui::backend::{Backend, IndexItem, IndexKind, Stub};
use quantus_ui::key::Key;
use quantus_ui::menu::{Edit, Kind};
use quantus_ui::render::Frame;
use quantus_ui::search::Target;
use quantus_ui::text::Lang;

/// The stub with a small index, as the engine gives it.
struct Library;

impl Backend for Library {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        Stub.evaluate(mode, settings, latex)
    }

    fn search_index(&mut self, _lang: Lang) -> Vec<IndexItem> {
        let item = |kind, latex: &str, names: &[&str], symbols: &[&str]| IndexItem {
            kind,
            latex: latex.to_string(),
            name: names[0].to_string(),
            names: names.iter().map(|name| name.to_string()).collect(),
            symbols: symbols.iter().map(|symbol| symbol.to_string()).collect(),
        };
        vec![
            item(IndexKind::Function, r"\sqrt{}", &["Quadratwurzel", "Square root", "Wurzel"], &["sqrt"]),
            item(IndexKind::Function, r"\sqrt[]{}", &["n-te Wurzel", "nth root"], &["root"]),
            item(IndexKind::Unit, r"\mathrm{in}", &["Zoll", "inch"], &["in"]),
        ]
    }
}

fn app() -> App<Library> {
    let mut app = App::new(Library);
    app.mode = Mode::Physics;
    app.settings.language = Lang::De;
    app
}

fn press<B: Backend>(app: &mut App<B>, keys: &[Key]) {
    for &key in keys {
        app.key(key);
        let _ = Frame::new(app);
    }
}

fn letters(text: &str) -> Vec<Key> {
    text.chars().map(Key::Letter).collect()
}

/// The names of the hits on the screen.
fn hits<B>(app: &App<B>) -> Vec<String> {
    let Screen::Menu(menu) = &app.screen else { panic!("CATALOG closed") };
    let Some(Edit::Catalogue { hits, .. }) = &menu.edit else { panic!("not CATALOG") };
    hits.iter().map(|hit| hit.name.clone()).collect()
}

#[test]
fn typing_in_catalog_finds_both_roots_and_inserts_the_first() {
    let mut app = app();
    press(&mut app, &[Key::Shift, Key::Optn]);
    press(&mut app, &letters("wurz"));
    let found = hits(&app);
    assert_eq!(&found[..2], ["Quadratwurzel", "n-te Wurzel"], "{found:?}");
    press(&mut app, &[Key::Exe]);
    assert_eq!(app.screen, Screen::Calc);
    assert_eq!(app.editor.to_latex(), r"\sqrt{}");
}

#[test]
fn a_unit_found_is_the_target_of_a_conversion() {
    let mut app = app();
    press(&mut app, &[Key::Shift, Key::Optn]);
    press(&mut app, &letters("zoll"));
    assert_eq!(hits(&app)[0], "Zoll");
    press(&mut app, &[Key::Exe]);
    assert!(app.editor.to_latex().contains(r"\mathrm{in}"), "{}", app.editor.to_latex());
}

#[test]
fn del_takes_back_a_letter_and_an_empty_query_shows_the_list_again() {
    let mut app = app();
    press(&mut app, &[Key::Shift, Key::Optn]);
    press(&mut app, &letters("wx"));
    assert!(hits(&app).is_empty());
    press(&mut app, &[Key::Del]);
    assert!(hits(&app).iter().any(|name| name.contains("Wurzel")));
    press(&mut app, &[Key::Del]);
    let Screen::Menu(menu) = &app.screen else { panic!("CATALOG closed") };
    assert!(matches!(&menu.edit, Some(Edit::Catalogue { query, group: Some(0), .. }) if query.is_empty()));
    // With nothing typed, DEL leaves CATALOG as before.
    press(&mut app, &[Key::Del]);
    assert_eq!(app.screen, Screen::Calc);
}

#[test]
fn a_setting_found_opens_where_it_lives() {
    let mut app = app();
    press(&mut app, &[Key::Shift, Key::Optn]);
    press(&mut app, &letters("zeitzone"));
    let Screen::Menu(menu) = &app.screen else { panic!("CATALOG closed") };
    let Some(Edit::Catalogue { hits, .. }) = &menu.edit else { panic!("not CATALOG") };
    assert!(matches!(hits[0].target, Target::Open(..)), "{:?}", hits[0]);
    press(&mut app, &[Key::Exe]);
    let Screen::Menu(menu) = &app.screen else { panic!("no menu opened") };
    // SETUP, down to the entry, with its submenu open or the entry under the cursor.
    assert_eq!(menu.stack[0].page.title.get(Lang::En), quantus_ui::tree::SETUP.title.get(Lang::En));
    let level = menu.level();
    let entry = &level.page.entries[level.selected];
    assert_eq!(entry.label.get(Lang::De), "Zeitzone");
    assert!(menu.edit.is_some() || matches!(entry.kind, Kind::Page(_)));
}

#[test]
fn without_an_engine_the_menus_are_still_found() {
    let mut app = App::new(Stub);
    app.mode = Mode::Algebra;
    app.settings.language = Lang::En;
    press(&mut app, &[Key::Shift, Key::Optn]);
    press(&mut app, &letters("factorial"));
    assert!(hits(&app).iter().any(|name| name.contains("Factorial")), "{:?}", hits(&app));
}
