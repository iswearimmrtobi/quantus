//! The menus of SETUP and OPTN: pages of entries, as on the Casio (design/handheld-menus.md §2).
//! Every entry that holds a value opens a submenu to change it: a list of its choices, or a
//! number typed with the digit keys. The tree itself is in [`crate::tree`].
//!
//! Keys: a digit picks the numbered entry of the page, ↑/↓ move, OK/EXE/→ open, ← or DEL go
//! back one level, AC closes the menu. Touch: a tap opens a line, a drag pages ([`Menu::tap`],
//! [`Menu::page`], [`Menu::step`]).

use crate::key::Key;
use crate::settings::{Setting, Settings};
use crate::text::{self, Lang, Text};

/// Entries per page on the screen: few enough that a line is easy to hit with a finger.
pub const ROWS: usize = 5;

#[derive(Debug, PartialEq, Eq)]
pub struct Page {
    pub title: Text,
    pub entries: &'static [Entry],
}

#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    pub label: Text,
    pub kind: Kind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Kind {
    /// A page of further entries.
    Page(&'static Page),
    /// A setting with a list of values.
    Choice(Setting, &'static [Opt]),
    /// A setting typed as a whole number in a range.
    Number(Setting, i32, i32),
    /// Something done after a *Ja*.
    Confirm(Action),
    /// Something done at once.
    Do(Action),
    /// A value to read.
    Info(Info),
    /// LaTeX for the input (OPTN).
    Insert(&'static str),
    /// The list of wallpapers on the SD card, which the app fills in.
    Wallpapers,
    /// Waits for the engine or the firmware: says so.
    Pending,
}

/// A value of a [`Kind::Choice`]; `then` asks for a number after it (Fix → n).
#[derive(Debug, PartialEq, Eq)]
pub struct Opt {
    pub text: Text,
    pub value: i32,
    pub then: Option<(Setting, i32, i32)>,
}

pub const fn opt(text: Text, value: i32) -> Opt {
    Opt { text, value, then: None }
}

/// On and off, as the value 1 and 0 of a switch.
pub static ON_OFF: [Opt; 2] = [opt(text::ON, 1), opt(text::OFF, 0)];

/// What the app does for an entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// The settings back to their defaults.
    ResetSettings,
    /// The history, Ans and the input cleared.
    ResetMemory,
    /// Both.
    ResetAll,
    /// The input cleared.
    ClearInput,
    /// The touch calibration screen.
    Calibrate,
}

/// Values a menu shows but cannot change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Info {
    /// The firmware's version.
    Version,
}

impl Info {
    pub fn value(self) -> String {
        match self {
            Info::Version => env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// A page on the way down, and its selected entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Level {
    pub page: &'static Page,
    pub selected: usize,
}

/// The submenu open over the page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Edit {
    Choice { entry: &'static Entry, selected: usize },
    /// `typed` is what was typed so far; `error` marks a value out of range.
    Number { label: Text, setting: Setting, min: i32, max: i32, typed: String, error: bool },
    Confirm { entry: &'static Entry, action: Action },
    /// A line to read; any key goes back.
    Message { label: Text, message: Text },
    /// The wallpapers: *Keins* first, then `names`. The app shows the selected one as a preview.
    Pictures { label: Text, names: Vec<String>, selected: usize },
}

/// A menu open on the screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Menu {
    /// From the root page to the one on the screen; never empty.
    pub stack: Vec<Level>,
    pub edit: Option<Edit>,
}

/// What the app has to do after a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Stay,
    Close,
    Insert(&'static str),
    Act(Action),
    /// Open the list of wallpapers over the page.
    Pictures(Text),
    /// The cursor is on the wallpaper at this place of the list (0 for none): show it.
    Preview(usize),
    /// Keep the wallpaper at this place of the list.
    Keep(usize),
    /// Back from the list: show the wallpaper set before.
    Restore,
}

impl Menu {
    pub fn new(root: &'static Page) -> Self {
        Menu { stack: vec![Level { page: root, selected: 0 }], edit: None }
    }

    pub fn level(&self) -> Level {
        *self.stack.last().expect("a menu has a page")
    }

    /// The title for the top bar: the page's, or the entry's whose submenu is open.
    pub fn title(&self) -> Text {
        match &self.edit {
            Some(Edit::Choice { entry, .. } | Edit::Confirm { entry, .. }) => entry.label,
            Some(Edit::Number { label, .. } | Edit::Message { label, .. } | Edit::Pictures { label, .. }) => *label,
            None => self.level().page.title,
        }
    }

    pub fn key(&mut self, key: Key, settings: &mut Settings) -> Outcome {
        if key == Key::Ac {
            return Outcome::Close;
        }
        match self.edit.take() {
            Some(edit) => self.edit_key(edit, key, settings),
            None => self.list_key(key, settings),
        }
    }

    fn list_key(&mut self, key: Key, settings: &mut Settings) -> Outcome {
        let level = self.level();
        let count = level.page.entries.len().max(1);
        let selected = match key {
            Key::Digit(digit @ 1..=9) => {
                let index = level.selected / ROWS * ROWS + usize::from(digit) - 1;
                if usize::from(digit) > ROWS || index >= count {
                    return Outcome::Stay;
                }
                self.select(index);
                return self.open(index, settings);
            }
            Key::Ok | Key::Exe | Key::Right => return self.open(level.selected, settings),
            Key::Left | Key::Del => {
                self.stack.pop();
                return if self.stack.is_empty() { Outcome::Close } else { Outcome::Stay };
            }
            Key::Mode | Key::Optn => return Outcome::Close,
            Key::Up => (level.selected + count - 1) % count,
            Key::Down => (level.selected + 1) % count,
            _ => level.selected,
        };
        self.select(selected);
        Outcome::Stay
    }

    /// A tap on the line `offset` of the list on the screen: opens or chooses it, as OK does.
    /// On the wallpapers a tap on another line only shows it, a second one keeps it.
    pub fn tap(&mut self, offset: usize, settings: &mut Settings) -> Outcome {
        if matches!(self.edit, Some(Edit::Confirm { .. })) {
            return if offset == 0 { self.key(Key::Ok, settings) } else { Outcome::Stay };
        }
        let pictures = matches!(self.edit, Some(Edit::Pictures { .. }));
        let Some((selected, count)) = self.list_mut() else { return Outcome::Stay };
        let index = *selected / ROWS * ROWS + offset;
        if offset >= ROWS || index >= count {
            return Outcome::Stay;
        }
        let again = *selected == index;
        *selected = index;
        if pictures && !again {
            return Outcome::Preview(index);
        }
        self.key(Key::Ok, settings)
    }

    /// The next page of the list on the screen, or the one before, on its first line.
    pub fn page(&mut self, forward: bool) -> Outcome {
        let pictures = matches!(self.edit, Some(Edit::Pictures { .. }));
        let Some((selected, count)) = self.list_mut() else { return Outcome::Stay };
        let first = *selected / ROWS * ROWS;
        let first = match forward {
            true => first + ROWS,
            false => match first.checked_sub(ROWS) {
                Some(first) => first,
                None => return Outcome::Stay,
            },
        };
        if first >= count {
            return Outcome::Stay;
        }
        *selected = first;
        if pictures {
            Outcome::Preview(first)
        } else {
            Outcome::Stay
        }
    }

    /// Steps the number being entered by `delta`, from the value set now if nothing is typed,
    /// kept in its range; OK sets it.
    pub fn step(&mut self, delta: i32, settings: &Settings) {
        let Some(Edit::Number { setting, min, max, typed, error, .. }) = &mut self.edit else { return };
        let now = typed.parse::<i32>().unwrap_or_else(|_| settings.get(*setting));
        *typed = now.saturating_add(delta).clamp(*min, *max).to_string();
        *error = false;
    }

    /// The cursor and the length of the list on the screen, if a list is.
    pub fn list(&self) -> Option<(usize, usize)> {
        match &self.edit {
            None => Some((self.level().selected, self.level().page.entries.len())),
            Some(Edit::Choice { entry, selected }) => match &entry.kind {
                Kind::Choice(_, opts) => Some((*selected, opts.len())),
                _ => None,
            },
            Some(Edit::Pictures { names, selected, .. }) => Some((*selected, names.len() + 1)),
            _ => None,
        }
    }

    /// [`Menu::list`], to move the cursor.
    fn list_mut(&mut self) -> Option<(&mut usize, usize)> {
        match &mut self.edit {
            None => {
                let level = self.stack.last_mut()?;
                Some((&mut level.selected, level.page.entries.len()))
            }
            Some(Edit::Choice { entry, selected }) => {
                let Kind::Choice(_, opts) = &entry.kind else { return None };
                Some((selected, opts.len()))
            }
            Some(Edit::Pictures { names, selected, .. }) => Some((selected, names.len() + 1)),
            _ => None,
        }
    }

    fn select(&mut self, selected: usize) {
        if let Some(level) = self.stack.last_mut() {
            level.selected = selected;
        }
    }

    /// Opens the entry `index` of the page on the screen.
    fn open(&mut self, index: usize, settings: &Settings) -> Outcome {
        let entry = &self.level().page.entries[index];
        match &entry.kind {
            Kind::Page(page) => self.stack.push(Level { page, selected: 0 }),
            Kind::Choice(setting, opts) => {
                let value = settings.get(*setting);
                let selected = opts.iter().position(|opt| opt.value == value).unwrap_or(0);
                self.edit = Some(Edit::Choice { entry, selected });
            }
            &Kind::Number(setting, min, max) => self.edit = Some(number(entry.label, setting, min, max)),
            &Kind::Confirm(action) => self.edit = Some(Edit::Confirm { entry, action }),
            &Kind::Do(action) => return Outcome::Act(action),
            Kind::Info(_) => {}
            Kind::Insert(latex) => return Outcome::Insert(latex),
            Kind::Wallpapers => return Outcome::Pictures(entry.label),
            Kind::Pending => {
                self.edit = Some(Edit::Message { label: entry.label, message: text::NOT_AVAILABLE });
            }
        }
        Outcome::Stay
    }

    fn edit_key(&mut self, edit: Edit, key: Key, settings: &mut Settings) -> Outcome {
        let back = matches!(key, Key::Left | Key::Mode | Key::Optn);
        match edit {
            Edit::Choice { entry, selected } => {
                let Kind::Choice(setting, opts) = &entry.kind else { return Outcome::Stay };
                let chosen = match key {
                    Key::Digit(digit @ 1..=9) if usize::from(digit) <= opts.len() => usize::from(digit) - 1,
                    Key::Ok | Key::Exe | Key::Right => selected,
                    Key::Up | Key::Down => {
                        let step = if key == Key::Up { opts.len() - 1 } else { 1 };
                        let selected = (selected + step) % opts.len();
                        self.edit = Some(Edit::Choice { entry, selected });
                        return Outcome::Stay;
                    }
                    Key::Del => return Outcome::Stay,
                    _ if back => return Outcome::Stay,
                    _ => {
                        self.edit = Some(Edit::Choice { entry, selected });
                        return Outcome::Stay;
                    }
                };
                let opt = &opts[chosen];
                if opt.value != 0 && settings.exam_blocks(*setting) {
                    self.edit = Some(Edit::Message { label: entry.label, message: text::EXAM_BLOCKS });
                    return Outcome::Stay;
                }
                settings.set(*setting, opt.value);
                if let Some((setting, min, max)) = opt.then {
                    self.edit = Some(number(opt.text, setting, min, max));
                }
            }
            Edit::Number { label, setting, min, max, mut typed, .. } => {
                let longest = min.to_string().len().max(max.to_string().len());
                match key {
                    Key::Digit(digit) if typed.len() < longest => typed.push(char::from(b'0' + digit.min(9))),
                    Key::Minus if min < 0 => {
                        if typed.starts_with('-') {
                            typed.remove(0);
                        } else {
                            typed.insert(0, '-');
                        }
                    }
                    Key::Del if !typed.is_empty() => {
                        typed.pop();
                    }
                    Key::Del => return Outcome::Stay,
                    Key::Ok | Key::Exe => {
                        if typed.is_empty() || typed == "-" {
                            return Outcome::Stay;
                        }
                        match typed.parse::<i32>() {
                            Ok(value) if (min..=max).contains(&value) => {
                                settings.set(setting, value);
                                return Outcome::Stay;
                            }
                            _ => {
                                let edit = Edit::Number { label, setting, min, max, typed, error: true };
                                self.edit = Some(edit);
                                return Outcome::Stay;
                            }
                        }
                    }
                    _ if back => return Outcome::Stay,
                    _ => {}
                }
                self.edit = Some(Edit::Number { label, setting, min, max, typed, error: false });
            }
            Edit::Confirm { entry, action } => match key {
                Key::Ok | Key::Exe | Key::Digit(1) => {
                    self.edit = Some(Edit::Message { label: entry.label, message: text::DONE });
                    return Outcome::Act(action);
                }
                Key::Del => {}
                _ if back => {}
                _ => self.edit = Some(Edit::Confirm { entry, action }),
            },
            Edit::Pictures { label, names, selected } => {
                let count = names.len() + 1;
                let chosen = match key {
                    Key::Digit(digit @ 1..=9) if usize::from(digit) <= ROWS => {
                        let index = selected / ROWS * ROWS + usize::from(digit) - 1;
                        if index >= count {
                            self.edit = Some(Edit::Pictures { label, names, selected });
                            return Outcome::Stay;
                        }
                        index
                    }
                    Key::Ok | Key::Exe | Key::Right => selected,
                    Key::Up | Key::Down => {
                        let step = if key == Key::Up { count - 1 } else { 1 };
                        let selected = (selected + step) % count;
                        self.edit = Some(Edit::Pictures { label, names, selected });
                        return Outcome::Preview(selected);
                    }
                    Key::Del => return Outcome::Restore,
                    _ if back => return Outcome::Restore,
                    _ => {
                        self.edit = Some(Edit::Pictures { label, names, selected });
                        return Outcome::Stay;
                    }
                };
                // The app reads the name from it, then closes it.
                self.edit = Some(Edit::Pictures { label, names, selected: chosen });
                return Outcome::Keep(chosen);
            }
            // Any key reads on.
            Edit::Message { .. } => {}
        }
        Outcome::Stay
    }
}

fn number(label: Text, setting: Setting, min: i32, max: i32) -> Edit {
    Edit::Number { label, setting, min, max, typed: String::new(), error: false }
}

/// The value an entry shows at the right of its line, if any.
pub fn value(entry: &Entry, settings: &Settings, lang: Lang) -> Option<String> {
    match &entry.kind {
        Kind::Choice(setting, opts) => {
            let value = settings.get(*setting);
            let opt = opts.iter().find(|opt| opt.value == value)?;
            let mut text = opt.text.get(lang).to_string();
            if let Some((then, _, _)) = opt.then {
                text = format!("{text} {}", settings.get(then));
            }
            Some(text)
        }
        Kind::Number(setting, min, _) => {
            let value = settings.get(*setting);
            Some(if *min < 0 && value > 0 { format!("+{value}") } else { value.to_string() })
        }
        Kind::Info(info) => Some(info.value()),
        _ => None,
    }
}
