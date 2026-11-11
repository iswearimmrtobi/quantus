//! The apps there are, and opening one from the list.

use quantus_apps::{Apps, Seeded};
use quantus_ui::text::Lang;

#[test]
fn the_built_in_apps_are_listed_and_open() {
    let apps = Apps::new(None);
    let list = apps.list(Lang::De);
    let counter = list.iter().find(|app| app.id == "zaehler").expect("the counter");
    assert_eq!(counter.name, "Zähler");
    assert!(counter.built_in && counter.usable);
    assert_eq!(apps.list(Lang::En).iter().find(|app| app.id == "zaehler").map(|app| app.name.as_str()), Some("Counter"));
    assert!(apps.open("zaehler").is_ok());
    assert!(apps.open("nothing").is_err());
}

#[test]
fn a_seed_repeats() {
    let (mut a, mut b) = (Seeded::new(7), Seeded::new(7));
    let first: Vec<u32> = (0..5).map(|_| a.next_u32()).collect();
    assert_eq!(first, (0..5).map(|_| b.next_u32()).collect::<Vec<_>>());
    assert_ne!(first[0], first[1]);
}
