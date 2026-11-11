//! The apps' data on the card: `/apps/data/<id>.json` (apps.md §2).

use std::path::PathBuf;

use quantus_apps::{Apps, ScriptApplet};
use quantus_ui::app::{App, Mode, Settings};
use quantus_ui::applet::AppInfo;
use quantus_ui::backend::{Backend, Stub};
use quantus_ui::key::Key;
use quantus_ui::render::{Frame, Shape};

/// An empty folder for one test.
fn folder(name: &str) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("quantus-apps-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a temporary folder");
    folder
}

#[test]
fn data_are_written_replaced_and_removed() {
    let card = folder("files");
    let apps = Apps::new(Some(card.clone()));
    assert_eq!(apps.data("g2048"), None);
    apps.store_data("g2048", "{\"best\":4}").expect("written");
    apps.store_data("g2048", "{\"best\":8}").expect("replaced");
    assert_eq!(apps.data("g2048").as_deref(), Some("{\"best\":8}"));
    assert!(card.join("data/g2048.json").exists());
    assert!(!card.join("data/g2048.new").exists());
    assert!(apps.delete_data("g2048"));
    assert!(!apps.delete_data("g2048"));
    // An id that is not a plain name never becomes a path.
    for bad in ["../evil", "a/b", "", "x.json", "card:x"] {
        assert!(apps.store_data(bad, "{}").is_err(), "{bad}");
        assert_eq!(apps.data(bad), None);
    }
    // Without a card nothing is kept.
    assert!(Apps::new(None).store_data("g2048", "{}").is_err());
}

/// The stub, keeping the apps' data in a folder.
struct Card(Apps);

impl Backend for Card {
    fn evaluate(&mut self, mode: Mode, settings: &Settings, latex: &str) -> Result<String, String> {
        Stub.evaluate(mode, settings, latex)
    }

    fn app_data(&mut self, id: &str) -> Option<String> {
        self.0.data(id)
    }

    fn store_app_data(&mut self, id: &str, json: &str) {
        self.0.store_data(id, json).expect("kept");
    }

    fn delete_app_data(&mut self, id: &str) -> bool {
        self.0.delete_data(id)
    }
}

const COUNTER: &str = r#"
    fn init() { let d = load(); this.n = if d == () { 0 } else { d.n }; }
    fn key(k) { this.n += 1; }
    fn leave() { save(#{ n: this.n }); }
    fn draw() { text(0, 0, `n ${this.n}`, "text"); }
"#;

fn open(app: &mut App<Card>) {
    let info = AppInfo { id: "zaehler".into(), name: "Zähler".into(), api: 1, usable: true, ..AppInfo::default() };
    app.open_applet(Box::new(ScriptApplet::new(COUNTER)), info);
}

fn texts(app: &App<Card>) -> Vec<String> {
    Frame::new(app).0.iter().filter_map(|shape| match shape {
        Shape::Text(_, text, ..) => Some(text.clone()),
        _ => None,
    }).collect()
}

#[test]
fn leave_saves_on_every_way_out_and_a_restart_finds_it() {
    let card = folder("leave");
    // AC, MODE, Beenden in the menu, each after a key.
    for way in [vec![Key::Ac], vec![Key::Mode], vec![Key::Optn, Key::Digit(4)]] {
        let mut app = App::new(Card(Apps::new(Some(card.clone()))));
        open(&mut app);
        app.key(Key::Digit(1));
        for key in way {
            app.key(key);
        }
    }
    // A new start of the handheld: the file has it.
    let mut app = App::new(Card(Apps::new(Some(card.clone()))));
    open(&mut app);
    assert!(texts(&app).contains(&"n 3".to_string()), "{:?}", texts(&app));
    // Daten löschen in the app's menu: after Ja, gone, and the app starts from nothing.
    for key in [Key::Optn, Key::Digit(3), Key::Ok] {
        app.key(key);
    }
    assert!(texts(&app).contains(&"n 0".to_string()), "{:?}", texts(&app));
    assert!(!card.join("data/zaehler.json").exists());
}
