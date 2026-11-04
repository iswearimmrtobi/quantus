//! The menus of SETUP and OPTN: pages of entries, as on the Casio (design/handheld-menus.md §2).
//! Every entry that holds a value opens a submenu to change it: a list of its choices, or a
//! number typed with the digit keys. The tree itself is in [`crate::tree`].
//!
//! Keys: a digit picks the numbered entry of the page, ↑/↓ move, OK/EXE/→ open, ← or DEL go
//! back one level, AC closes the menu. Touch: a tap opens a line, a drag pages ([`Menu::tap`],
//! [`Menu::page`], [`Menu::step`]).

use crate::backend::{Catalogue, Group, Network, SessionInfo, Storage, WlanStatus};
use crate::clock::{self, Local};
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
    /// How full the SD card is, which the app asks the board.
    Storage,
    /// The list of sessions, which the app fills in; choosing one opens it.
    Sessions,
    /// The pages of the last curve sketch, which the app fills in; choosing one shows it.
    Sections,
    /// A new name for the open session, typed.
    Rename,
    /// The date or the time of day, typed: sets the clock.
    Time(TimeField),
    /// The WLAN networks in reach, which the app has the board look for; choosing one connects.
    Networks,
    /// The networks connected to before, with their passwords; choosing one forgets it.
    SavedNetworks,
    /// The WLAN connection: the network, the address, the signal.
    WlanStatus,
    /// The constants or the units by category, which the app asks the backend for; choosing
    /// one inserts it.
    Catalogue(Catalogue),
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
    /// The input cleared, or the sheet on the screen.
    ClearInput,
    /// An empty row above the cursor of a list (Statistik).
    InsertRow,
    /// The cursor's row of a list taken out.
    DeleteRow,
    /// The touch calibration screen.
    Calibrate,
    /// A new session, opened.
    NewSession,
    /// A copy of the open session, opened.
    DuplicateSession,
    /// The open session deleted; the next one opens.
    DeleteSession,
    /// Kurvendiskussion's form for the tangent and the normal at x₀.
    TangentForm,
    /// Its form for the area between a and b.
    AreaForm,
    /// Its form for a table of values.
    PointsForm,
    /// CALC: the input worked out for values of its letters, asked for in a form.
    Calc,
    /// SOLVE: the input solved for the letter whose field is left empty.
    Solve,
}

impl Action {
    /// Whether it changes the open session: the app closes the menu afterwards.
    pub fn session(self) -> bool {
        matches!(self, Action::NewSession | Action::DuplicateSession | Action::DeleteSession)
    }
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

/// A part of the clock that is typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeField {
    /// TT.MM.JJJJ
    Date,
    /// HH:MM
    Clock,
}

/// The years a date may be typed in.
const YEARS: std::ops::RangeInclusive<i32> = 2025..=2099;

impl TimeField {
    fn digits(self) -> usize {
        match self {
            TimeField::Date => 8,
            TimeField::Clock => 4,
        }
    }

    /// `typed` in the field's form, `_` for the digits still to come: `02.11.____`.
    pub fn show(self, typed: &str) -> String {
        let form = match self {
            TimeField::Date => "__.__.____",
            TimeField::Clock => "__:__",
        };
        let mut digits = typed.chars();
        form.chars().map(|c| if c == '_' { digits.next().unwrap_or('_') } else { c }).collect()
    }

    /// The hint under the field, and what an invalid entry is told.
    pub fn hint(self, error: bool) -> Text {
        match (self, error) {
            (TimeField::Date, false) => text::DATE_HINT,
            (TimeField::Date, true) => text::BAD_DATE,
            (TimeField::Clock, false) => text::CLOCK_HINT,
            (TimeField::Clock, true) => text::BAD_CLOCK,
        }
    }

    /// The local time `now` in this field's form; the date with its weekday.
    pub fn format(self, now: &Local, lang: Lang) -> String {
        match self {
            TimeField::Date => {
                let weekday = text::WEEKDAYS[usize::from(now.weekday)].get(lang);
                format!("{weekday} {:02}.{:02}.{}", now.day, now.month, now.year)
            }
            TimeField::Clock => format!("{:02}:{:02}", now.hour, now.minute),
        }
    }
}

/// A date typed as TT.MM.JJJJ: year, month and day, if it is a real one.
pub fn typed_date(typed: &str) -> Option<(i32, u8, u8)> {
    if typed.len() != 8 {
        return None;
    }
    let day: u8 = typed[0..2].parse().ok()?;
    let month: u8 = typed[2..4].parse().ok()?;
    let year: i32 = typed[4..8].parse().ok()?;
    let real = YEARS.contains(&year) && (1..=clock::days_in_month(year, month)).contains(&day);
    real.then_some((year, month, day))
}

/// A time typed as HH:MM: hour and minute, if it is a real one.
pub fn typed_clock(typed: &str) -> Option<(u8, u8)> {
    if typed.len() != 4 {
        return None;
    }
    let hour: u8 = typed[0..2].parse().ok()?;
    let minute: u8 = typed[2..4].parse().ok()?;
    (hour < 24 && minute < 60).then_some((hour, minute))
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
    /// A text to read that the app made, the engine's error: any key goes back.
    Note { label: Text, note: String },
    /// The wallpapers: *Keins* first, then `names`. The app shows the selected one as a preview.
    Pictures { label: Text, names: Vec<String>, selected: usize },
    /// The SD card's size and room, `None` without one; any key goes back.
    Storage { label: Text, storage: Option<Storage> },
    /// The sessions; the app opens the chosen one.
    Sessions { label: Text, sessions: Vec<SessionInfo>, selected: usize },
    /// The pages of the last curve sketch by their titles; the app shows the chosen one.
    Sections { label: Text, titles: Vec<String>, selected: usize },
    /// A line of text being typed, for `purpose`; `error` marks one it does not take, `plain`
    /// shows a password as typed (S⇔D) instead of stars.
    Text { label: Text, purpose: TextFor, typed: String, error: bool, plain: bool },
    /// The networks found, `None` while the board still looks.
    Networks { label: Text, networks: Option<Vec<Network>>, selected: usize },
    /// The networks saved.
    Saved { label: Text, names: Vec<String>, selected: usize },
    /// *Vergessen* for the saved network `ssid`.
    Forget { label: Text, ssid: String },
    /// The connection as the board has it now; any key goes back.
    WlanStatus { label: Text, status: WlanStatus },
    /// The digits of a date or a time being typed; `error` marks one that does not exist.
    Time { label: Text, field: TimeField, typed: String, error: bool },
    /// The categories of a catalogue, or the items of the category `group`; `close` says that
    /// leaving the categories closes the menu (opened by its key, not from a page).
    Catalogue { label: Text, catalogue: Catalogue, groups: Vec<Group>, group: Option<usize>, selected: usize, close: bool },
}

/// The longest name a session can be given on the handheld.
pub const NAME_LENGTH: usize = 24;

/// A WPA password's length.
pub const PASSWORD_LENGTH: std::ops::RangeInclusive<usize> = 8..=63;

/// What a text field is for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextFor {
    /// The open session's new name.
    Name,
    /// The password of the network `ssid`.
    Password(String),
    /// The name a value is kept under (STO): the value as LaTeX.
    Variable(String),
}

/// The longest name of a variable typed for STO.
pub const VARIABLE_LENGTH: usize = 12;

/// Whether `name` names a variable: a letter, then letters or digits, and an index after `_`.
pub fn variable_name(name: &str) -> bool {
    let (letters, index) = name.split_once('_').map_or((name, None), |(letters, index)| (letters, Some(index)));
    let alphanumeric = |part: &str| part.chars().all(|c| c.is_ascii_alphanumeric());
    letters.starts_with(|c: char| c.is_ascii_alphabetic())
        && alphanumeric(letters)
        && index.is_none_or(|index| !index.is_empty() && alphanumeric(index))
}

impl TextFor {
    fn longest(&self) -> usize {
        match self {
            TextFor::Name => NAME_LENGTH,
            TextFor::Password(_) => *PASSWORD_LENGTH.end(),
            TextFor::Variable(_) => VARIABLE_LENGTH,
        }
    }

    /// Whether `typed` is taken: a name not empty, a password of WPA's length.
    fn takes(&self, typed: &str) -> bool {
        match self {
            TextFor::Name => !typed.trim().is_empty(),
            TextFor::Password(_) => PASSWORD_LENGTH.contains(&typed.chars().count()),
            TextFor::Variable(_) => variable_name(typed),
        }
    }
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
    /// Show how full the SD card is.
    Storage(Text),
    /// Open the list of sessions over the page.
    Sessions(Text),
    /// Open the session at this place of the list.
    OpenSession(usize),
    /// Open the list of the sketch's pages over the page.
    Sections(Text),
    /// Show the page of the sketch at this place of the list.
    Section(usize),
    /// Open the name entry over the page, with the open session's name.
    NameOf(Text),
    /// Give the open session the name typed (the app reads it from the edit).
    Rename,
    /// Look for networks and list them over the page.
    Networks(Text),
    /// Connect to the network at this place of the list.
    Network(usize),
    /// Connect with the password typed (the app reads it and the network from the edit).
    Connect,
    /// Open the list of saved networks over the page.
    SavedNetworks(Text),
    /// Forget the saved network of the edit.
    Forget,
    /// Show the connection.
    WlanStatus(Text),
    /// Set the clock to the date or time typed, which is a real one (the app reads it from the
    /// edit).
    SetTime(TimeField),
    /// Open a catalogue over the page.
    Catalogue(Text, Catalogue),
    /// Insert the item under the cursor of the catalogue (the app reads it from the edit).
    Item,
    /// Keep the value of the text field under the name typed (the app reads both from it).
    Store,
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
            Some(
                Edit::Number { label, .. }
                | Edit::Message { label, .. }
                | Edit::Note { label, .. }
                | Edit::Pictures { label, .. }
                | Edit::Storage { label, .. }
                | Edit::Sessions { label, .. }
                | Edit::Sections { label, .. }
                | Edit::Text { label, .. }
                | Edit::Time { label, .. }
                | Edit::Networks { label, .. }
                | Edit::Saved { label, .. }
                | Edit::Forget { label, .. }
                | Edit::WlanStatus { label, .. }
                | Edit::Catalogue { label, .. },
            ) => *label,
            None => self.level().page.title,
        }
    }

    pub fn key(&mut self, key: Key, settings: &mut Settings) -> Outcome {
        if key == Key::Ac {
            return Outcome::Close;
        }
        let outcome = match self.edit.take() {
            Some(edit) => self.edit_key(edit, key, settings),
            None => self.list_key(key, settings),
        };
        // A page without entries only holds a submenu (STO's): leaving it closes the menu.
        let bare = self.stack.last().is_some_and(|level| level.page.entries.is_empty());
        if bare && self.edit.is_none() && outcome == Outcome::Stay {
            return Outcome::Close;
        }
        outcome
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
        if matches!(self.edit, Some(Edit::Confirm { .. } | Edit::Forget { .. })) {
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
            Some(Edit::Sessions { sessions, selected, .. }) => Some((*selected, sessions.len())),
            Some(Edit::Sections { titles, selected, .. }) => Some((*selected, titles.len())),
            Some(Edit::Networks { networks: Some(networks), selected, .. }) => Some((*selected, networks.len())),
            Some(Edit::Saved { names, selected, .. }) => Some((*selected, names.len())),
            Some(Edit::Catalogue { groups, group, selected, .. }) => Some((*selected, catalogue_len(groups, *group))),
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
            Some(Edit::Sessions { sessions, selected, .. }) => Some((selected, sessions.len())),
            Some(Edit::Sections { titles, selected, .. }) => Some((selected, titles.len())),
            Some(Edit::Networks { networks: Some(networks), selected, .. }) => Some((selected, networks.len())),
            Some(Edit::Saved { names, selected, .. }) => Some((selected, names.len())),
            Some(Edit::Catalogue { groups, group, selected, .. }) => Some((selected, catalogue_len(groups, *group))),
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
            Kind::Storage => return Outcome::Storage(entry.label),
            Kind::Sessions => return Outcome::Sessions(entry.label),
            Kind::Sections => return Outcome::Sections(entry.label),
            Kind::Rename => return Outcome::NameOf(entry.label),
            Kind::Networks => return Outcome::Networks(entry.label),
            Kind::SavedNetworks => return Outcome::SavedNetworks(entry.label),
            Kind::WlanStatus => return Outcome::WlanStatus(entry.label),
            &Kind::Catalogue(catalogue) => return Outcome::Catalogue(entry.label, catalogue),
            &Kind::Time(field) => {
                self.edit = Some(Edit::Time { label: entry.label, field, typed: String::new(), error: false });
            }
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
                    // The numbered line of the page on the screen, as in the other lists.
                    Key::Digit(digit @ 1..=9) if usize::from(digit) <= ROWS => {
                        let index = selected / ROWS * ROWS + usize::from(digit) - 1;
                        if index >= opts.len() {
                            self.edit = Some(Edit::Choice { entry, selected });
                            return Outcome::Stay;
                        }
                        index
                    }
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
            Edit::Sessions { label, sessions, selected } => {
                let count = sessions.len();
                let chosen = match key {
                    Key::Digit(digit @ 1..=9) if usize::from(digit) <= ROWS => {
                        let index = selected / ROWS * ROWS + usize::from(digit) - 1;
                        if index >= count {
                            self.edit = Some(Edit::Sessions { label, sessions, selected });
                            return Outcome::Stay;
                        }
                        index
                    }
                    Key::Ok | Key::Exe | Key::Right if selected < count => selected,
                    Key::Up | Key::Down if count > 0 => {
                        let step = if key == Key::Up { count - 1 } else { 1 };
                        let selected = (selected + step) % count;
                        self.edit = Some(Edit::Sessions { label, sessions, selected });
                        return Outcome::Stay;
                    }
                    Key::Del => return Outcome::Stay,
                    _ if back => return Outcome::Stay,
                    _ => {
                        self.edit = Some(Edit::Sessions { label, sessions, selected });
                        return Outcome::Stay;
                    }
                };
                // The app reads the id from it, then closes the menu.
                self.edit = Some(Edit::Sessions { label, sessions, selected: chosen });
                return Outcome::OpenSession(chosen);
            }
            Edit::Sections { label, titles, selected } => {
                let count = titles.len();
                let chosen = match key {
                    Key::Digit(digit @ 1..=9) if selected / ROWS * ROWS + usize::from(digit) - 1 < count => {
                        selected / ROWS * ROWS + usize::from(digit) - 1
                    }
                    Key::Ok | Key::Exe | Key::Right if selected < count => selected,
                    Key::Up | Key::Down if count > 0 => {
                        let step = if key == Key::Up { count - 1 } else { 1 };
                        self.edit = Some(Edit::Sections { label, titles, selected: (selected + step) % count });
                        return Outcome::Stay;
                    }
                    _ if back || key == Key::Del => return Outcome::Stay,
                    _ => {
                        self.edit = Some(Edit::Sections { label, titles, selected });
                        return Outcome::Stay;
                    }
                };
                return Outcome::Section(chosen);
            }
            Edit::Text { label, purpose, mut typed, plain, .. } => {
                match key {
                    Key::Del if !typed.is_empty() => {
                        typed.pop();
                    }
                    Key::Del => return Outcome::Stay,
                    Key::SwapForm => {
                        self.edit = Some(Edit::Text { label, purpose, typed, error: false, plain: !plain });
                        return Outcome::Stay;
                    }
                    Key::Ok | Key::Exe => {
                        let error = !purpose.takes(&typed);
                        let outcome = match purpose {
                            _ if error => Outcome::Stay,
                            TextFor::Name => Outcome::Rename,
                            TextFor::Password(_) => Outcome::Connect,
                            TextFor::Variable(_) => Outcome::Store,
                        };
                        self.edit = Some(Edit::Text { label, purpose, typed, error, plain });
                        return outcome;
                    }
                    _ if back => return Outcome::Stay,
                    _ => {
                        if let Some(c) = key.text().filter(|_| typed.chars().count() < purpose.longest()) {
                            typed.push(c);
                        }
                    }
                }
                self.edit = Some(Edit::Text { label, purpose, typed, error: false, plain });
            }
            Edit::Networks { label, networks: Some(networks), selected } => {
                match pick(key, selected, networks.len()) {
                    Pick::Move(selected) => self.edit = Some(Edit::Networks { label, networks: Some(networks), selected }),
                    Pick::Chosen(chosen) => {
                        // The app reads the network from it.
                        self.edit = Some(Edit::Networks { label, networks: Some(networks), selected: chosen });
                        return Outcome::Network(chosen);
                    }
                    Pick::Back => {}
                }
            }
            Edit::Saved { label, names, selected } => match pick(key, selected, names.len()) {
                Pick::Move(selected) => self.edit = Some(Edit::Saved { label, names, selected }),
                Pick::Chosen(chosen) => self.edit = Some(Edit::Forget { label, ssid: names[chosen].clone() }),
                Pick::Back => {}
            },
            Edit::Forget { label, ssid } => match key {
                Key::Ok | Key::Exe | Key::Digit(1) => {
                    // The app reads the network from it.
                    self.edit = Some(Edit::Forget { label, ssid });
                    return Outcome::Forget;
                }
                Key::Del => {}
                _ if back => {}
                _ => self.edit = Some(Edit::Forget { label, ssid }),
            },
            Edit::Time { label, field, mut typed, .. } => {
                match key {
                    Key::Digit(digit) if typed.len() < field.digits() => typed.push(char::from(b'0' + digit.min(9))),
                    Key::Del if !typed.is_empty() => {
                        typed.pop();
                    }
                    Key::Del => return Outcome::Stay,
                    Key::Ok | Key::Exe => {
                        let real = match field {
                            TimeField::Date => typed_date(&typed).is_some(),
                            TimeField::Clock => typed_clock(&typed).is_some(),
                        };
                        self.edit = Some(Edit::Time { label, field, typed, error: !real });
                        return if real { Outcome::SetTime(field) } else { Outcome::Stay };
                    }
                    _ if back => return Outcome::Stay,
                    _ => {}
                }
                self.edit = Some(Edit::Time { label, field, typed, error: false });
            }
            Edit::Catalogue { label, catalogue, groups, group, selected, close } => {
                let edit = |groups, group, selected| Some(Edit::Catalogue { label, catalogue, groups, group, selected, close });
                match (pick(key, selected, catalogue_len(&groups, group)), group) {
                    (Pick::Move(selected), _) => self.edit = edit(groups, group, selected),
                    (Pick::Chosen(chosen), None) => self.edit = edit(groups, Some(chosen), 0),
                    (Pick::Chosen(chosen), Some(_)) => {
                        // The app reads the item from it.
                        self.edit = edit(groups, group, chosen);
                        return Outcome::Item;
                    }
                    (Pick::Back, Some(index)) => self.edit = edit(groups, None, index),
                    (Pick::Back, None) if close => return Outcome::Close,
                    (Pick::Back, None) => {}
                }
            }
            // Any key reads on; so it does while the board looks for networks.
            Edit::Message { .. }
            | Edit::Note { .. }
            | Edit::Storage { .. }
            | Edit::WlanStatus { .. }
            | Edit::Networks { networks: None, .. } => {}
        }
        Outcome::Stay
    }
}

/// What a key does in a list the app filled in.
enum Pick {
    /// The cursor moved to here, or stays.
    Move(usize),
    Chosen(usize),
    Back,
}

/// A key in a list of `count` lines with the cursor on `selected`: a digit picks the numbered
/// line of the page, OK the one under the cursor, ↑/↓ move, ← and DEL go back.
fn pick(key: Key, selected: usize, count: usize) -> Pick {
    match key {
        Key::Digit(digit @ 1..=9) if usize::from(digit) <= ROWS => {
            let index = selected / ROWS * ROWS + usize::from(digit) - 1;
            if index < count { Pick::Chosen(index) } else { Pick::Move(selected) }
        }
        Key::Ok | Key::Exe | Key::Right if selected < count => Pick::Chosen(selected),
        Key::Up | Key::Down if count > 0 => {
            let step = if key == Key::Up { count - 1 } else { 1 };
            Pick::Move((selected + step) % count)
        }
        Key::Del | Key::Left | Key::Mode | Key::Optn => Pick::Back,
        _ => Pick::Move(selected),
    }
}

/// The lines of a catalogue: its categories, or the items of the category `group`.
fn catalogue_len(groups: &[Group], group: Option<usize>) -> usize {
    match group {
        None => groups.len(),
        Some(index) => groups.get(index).map_or(0, |group| group.items.len()),
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
        &Kind::Time(field) => Some(match clock::now() {
            Some(now) => field.format(&clock::local(now, settings.zone), lang),
            None => text::NOT_SET.get(lang).to_string(),
        }),
        _ => None,
    }
}
