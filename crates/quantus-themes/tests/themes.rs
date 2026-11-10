use quantus_themes::{built_in, default_theme, find, list, parse, Color, BUILT_IN, COLORS, HANDHELD, TILES};

#[test]
fn built_in_themes_are_complete_and_without_warnings() {
    for (id, _) in BUILT_IN {
        let theme = built_in(id).unwrap();
        assert_eq!(theme.colors.len(), COLORS.len(), "{id}");
        assert_eq!(theme.handheld.colors.len(), HANDHELD.len(), "{id}");
        assert!(theme.warnings.is_empty(), "{id}: {:?}", theme.warnings);
        assert_eq!(theme.handheld.tiles.len(), TILES);
    }
    assert!(built_in("dark").unwrap().dark);
    assert!(!built_in("light").unwrap().dark);
    assert_eq!(default_theme().id, "dark");
}

/// The WCAG contrast ratio of two opaque colours, from 1 to 21.
fn contrast(a: Color, b: Color) -> f64 {
    let luminance = |c: Color| {
        let channel = |v: u8| {
            let v = f64::from(v) / 255.0;
            if v <= 0.040_45 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
    };
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[test]
fn built_in_themes_have_readable_text() {
    for (id, _) in BUILT_IN {
        let theme = built_in(id).unwrap();
        let least = if id.starts_with("contrast-") { 7.0 } else { 4.5 };
        for text in ["text", "text-muted"] {
            for ground in ["background", "surface"] {
                let ratio = contrast(theme.color(text), theme.color(ground));
                assert!(ratio >= least, "{id}: {text} on {ground} is {ratio:.2}:1, needs {least}:1");
            }
        }
    }
}

#[test]
fn names_are_translated_and_fall_back() {
    let light = built_in("light").unwrap();
    assert_eq!(light.name("de"), "Hell");
    assert_eq!(light.name("en"), "Light");
    assert_eq!(light.name("fr"), "Light");
    let plain = parse("plain", "name = \"Plain\"").unwrap();
    assert_eq!(plain.name("de"), "Plain");
    let nameless = parse("nameless", "").unwrap();
    assert_eq!(nameless.name("en"), "nameless");
}

#[test]
fn a_theme_starts_from_its_base() {
    let dark = built_in("dark").unwrap();
    let light = built_in("light").unwrap();

    let theme = parse("mine", "[colors]\nbackground = \"#000000\"").unwrap();
    assert_eq!(theme.color("background"), Color::rgb(0, 0, 0));
    assert_eq!(theme.color("text"), dark.color("text"));
    assert_eq!(theme.handheld.tiles, dark.handheld.tiles);
    assert!(theme.dark);

    let theme = parse("mine", "base = \"light\"\n[handheld]\nresult = \"#102030\"").unwrap();
    assert!(!theme.dark);
    assert_eq!(theme.color("background"), light.color("background"));
    assert_eq!(theme.handheld("result"), Color::rgb(0x10, 0x20, 0x30));
    assert_eq!(theme.handheld("unit"), light.handheld("unit"));
}

#[test]
fn colours_may_have_an_opacity() {
    let theme = parse("mine", "[colors]\nglow = \"#FFFFFF80\"").unwrap();
    assert_eq!(theme.color("glow"), Color { r: 255, g: 255, b: 255, a: 128 });
    let vars = theme.css_vars();
    assert!(vars.contains(&("--glow".to_string(), "#ffffff80".to_string())));
    assert!(vars.contains(&("--background".to_string(), "#17161d".to_string())));
    assert_eq!(vars.len(), COLORS.len());
}

#[test]
fn broken_files_say_why() {
    let error = |text: &str| parse("bad", text).unwrap_err().0;
    assert!(error("[colors]\ntext = \"red\"").contains("colors.text"));
    assert!(error("[colors]\ntext = \"#12345\"").contains("not a colour"));
    assert!(error("[colors]\ntext = 3").contains("colors.text"));
    assert!(error("base = \"nope\"").contains("nope"));
    assert!(error("dark = \"yes\"").contains("dark"));
    assert!(error("[handheld]\ntiles = [[\"#000000\", \"#000000\"]]").contains("handheld.tiles"));
    assert!(error("colors = 1").contains("[colors]"));
    assert!(!error("[colors\n").is_empty());
}

#[test]
fn unknown_keys_are_warnings() {
    let theme = parse("mine", "author = \"me\"\n[colors]\nbackgrund = \"#000000\"\n[handheld]\nfoo = \"#000000\"").unwrap();
    assert_eq!(theme.warnings.len(), 3, "{:?}", theme.warnings);
    assert!(theme.warnings.iter().any(|w| w.contains("colors.backgrund")));
}

#[test]
fn users_themes_follow_the_built_in_ones_and_may_replace_them() {
    let user = vec![
        ("zebra".to_string(), "name = \"Zebra\"".to_string()),
        ("broken".to_string(), "[colors]\ntext = \"x\"".to_string()),
        ("light".to_string(), "name = \"My light\"\nbase = \"light\"".to_string()),
    ];
    let entries = list(user.clone());
    let ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["dark", "light", "black", "paper", "contrast-dark", "contrast-light", "broken", "zebra"]);
    assert!(!entries[1].built_in);
    assert_eq!(entries[1].theme.as_ref().unwrap().name("en"), "My light");
    assert!(entries[6].theme.is_err());

    assert_eq!(find("light", user.clone()).unwrap().name("en"), "My light");
    assert_eq!(find("dark", user.clone()).unwrap().name("en"), "Dark");
    assert!(find("missing", user).is_err());
}
