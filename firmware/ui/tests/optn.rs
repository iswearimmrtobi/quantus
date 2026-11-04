//! OPTN's entries insert what the engine reads, at the cursor.

use quantus_ui::app::{App, Mode, Screen};
use quantus_ui::backend::Stub;
use quantus_ui::key::Key;
use quantus_ui::text::Lang;

/// Opens OPTN and goes down the entries labelled `path` (German), choosing the last.
fn optn(app: &mut App<Stub>, path: &[&str]) {
    app.key(Key::Optn);
    for label in path {
        let Screen::Menu(menu) = &app.screen else { panic!("OPTN closed before {label:?}") };
        let page = menu.level().page;
        let index = page
            .entries
            .iter()
            .position(|entry| entry.label.get(Lang::De) == *label)
            .unwrap_or_else(|| panic!("no {label:?} on {:?}", page.title.get(Lang::De)));
        let selected = menu.level().selected;
        let steps = (index + page.entries.len() - selected) % page.entries.len();
        (0..steps).for_each(|_| app.key(Key::Down));
        app.key(Key::Exe);
    }
}

fn app(mode: Mode) -> App<Stub> {
    let mut app = App::new(Stub);
    app.mode = mode;
    app
}

/// What `path` inserts into an empty input in Physik.
fn inserted(path: &[&str]) -> String {
    let mut app = app(Mode::Physics);
    optn(&mut app, path);
    assert_eq!(app.screen, Screen::Calc, "{path:?} closes OPTN");
    app.editor.to_latex()
}

#[test]
fn the_area_functions_and_coth_go_into_their_brackets() {
    let mut app = app(Mode::Physics);
    optn(&mut app, &["Hyperbol. Fkt.", "arsinh"]);
    app.key(Key::Digit(1));
    assert_eq!(app.editor.to_latex(), r"\mathrm{arsinh}(1)");
    assert_eq!(inserted(&["Hyperbol. Fkt.", "arcosh"]), r"\mathrm{arcosh}()");
    assert_eq!(inserted(&["Hyperbol. Fkt.", "artanh"]), r"\mathrm{artanh}()");
    assert_eq!(inserted(&["Hyperbol. Fkt.", "coth"]), r"\coth()");
}
