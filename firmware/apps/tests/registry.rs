//! The apps there are, and opening one from the list.

use quantus_apps::{Apps, Seeded};
use quantus_ui::text::Lang;

#[test]
fn the_built_in_apps_are_listed_and_open() {
    let apps = Apps::new(None);
    let list = apps.list(Lang::De);
    let ids: Vec<&str> = list.iter().map(|app| app.id.as_str()).collect();
    assert_eq!(ids, ["2048", "tictactoe"]);
    let game = list.iter().find(|app| app.id == "tictactoe").expect("tic tac toe");
    assert_eq!(game.name, "Tic Tac Toe");
    assert!(game.built_in && game.usable);
    assert_eq!(apps.list(Lang::En).iter().find(|app| app.id == "tictactoe").map(|app| app.name.as_str()), Some("Tic tac toe"));
    assert!(apps.open("tictactoe").is_ok());
    assert!(apps.open("nothing").is_err());
}

#[test]
fn a_seed_repeats() {
    let (mut a, mut b) = (Seeded::new(7), Seeded::new(7));
    let first: Vec<u32> = (0..5).map(|_| a.next_u32()).collect();
    assert_eq!(first, (0..5).map(|_| b.next_u32()).collect::<Vec<_>>());
    assert_ne!(first[0], first[1]);
}
