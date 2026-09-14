//! Table-driven key sequences: `j`, `gg`, `3j`, `<C-k>`.
//!
//! iced hands an application one key press at a time, which is enough for accelerators and not
//! enough for a modal interface: `g i` is two presses that mean one thing, and `3 j` is a number
//! attached to a third. A [`Keymap`] is a table of specs to actions; a [`Pending`] is the few
//! keystrokes the user is part-way through.
//!
//! ```
//! use iced::keyboard::{Key, Modifiers};
//! use noctalia_iced::keymap::{Keymap, Pending, Resolved};
//!
//! #[derive(Debug, Clone, PartialEq)]
//! enum Action { Down, Top, Palette }
//!
//! let keys = Keymap::new()
//!     .counted()
//!     .bind("j", Action::Down)
//!     .bind("gg", Action::Top)
//!     .bind("<C-k>", Action::Palette);
//!
//! let mut pending = Pending::default();
//! assert!(matches!(keys.press(&mut pending, &Key::Character("3".into()), Modifiers::empty()), Resolved::Pending));
//! let resolved = keys.press(&mut pending, &Key::Character("j".into()), Modifiers::empty());
//! assert_eq!(resolved, Resolved::Action(Action::Down, 3));
//! ```
//!
//! # Mode lives in the application
//!
//! There is no mode here on purpose. An application holds as many [`Keymap`]s as it has focus
//! regions and picks one per press, which is both simpler than a mode stack and the thing it
//! actually wants: the pager's `j` and the index's `j` are different actions, not one action asking
//! where it is. Insert mode needs nothing at all — iced reports a press a focused text input
//! consumed as [`Captured`](iced::event::Status::Captured), so filtering on that keeps typing out
//! of the keymap without either side knowing about the other.
//!
//! # Prefixes resolve immediately
//!
//! A sequence that is both a whole binding and the start of a longer one (`g` alongside `gg`)
//! resolves as the short one the moment it matches: waiting for the rest would need a timer, and a
//! timer is worse than the rule "don't bind a prefix of another binding". [`Keymap::conflicts`]
//! reports any such pair, so a test can hold a table to that rule.

use iced::keyboard::{Key, Modifiers, key::Named};

/// One key, without the modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stroke {
    /// A character, as typed: shift is already in it, so `J` and `j` are different strokes.
    Char(char),
    Named(Named),
}

/// One key with the modifiers it needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Press {
    pub stroke: Stroke,
    /// Control, alt and logo. Shift is not here for a [`Stroke::Char`] — it is in the character.
    pub modifiers: Modifiers,
}

/// Modifiers that are part of the binding rather than part of the character.
const CHORD: Modifiers = Modifiers::CTRL.union(Modifiers::ALT).union(Modifiers::LOGO);

impl Press {
    /// The press iced just reported, or `None` for a key no binding could name — a bare modifier,
    /// or a character that arrived without one.
    pub fn of(key: &Key, modifiers: Modifiers) -> Option<Press> {
        let stroke = match key.as_ref() {
            Key::Character(text) => Stroke::Char(text.chars().next()?),
            Key::Named(named) => Stroke::Named(named),
            Key::Unidentified => return None,
        };
        // Shift is significant for a named key (`<S-Tab>`) and already spent on a character.
        let mask = match stroke {
            Stroke::Char(_) => CHORD,
            Stroke::Named(_) => CHORD.union(Modifiers::SHIFT),
        };
        Some(Press { stroke, modifiers: modifiers.intersection(mask) })
    }
}

/// What a press did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved<A> {
    /// No binding starts this way. The application may do what it likes with the key; anything
    /// half-typed has been thrown away.
    Ignored,
    /// Part-way through a sequence, or building a count. Swallow the key.
    Pending,
    /// A binding fired, with its count — 1 when none was typed.
    Action(A, u32),
}

/// The few keystrokes the user is part-way through. One per focus region, held by the application.
#[derive(Debug, Clone, Default)]
pub struct Pending {
    count: Option<u32>,
    keys: Vec<Press>,
}

impl Pending {
    /// The count typed so far, for an interface that shows it.
    pub fn count(&self) -> Option<u32> {
        self.count
    }

    /// The keys typed so far, for an interface that shows them.
    pub fn keys(&self) -> &[Press] {
        &self.keys
    }

    /// Whether anything is half-typed. Worth drawing when true, and worth clearing on Escape.
    pub fn is_active(&self) -> bool {
        self.count.is_some() || !self.keys.is_empty()
    }

    /// What has been typed so far, written the way a binding is. For the corner of a status bar.
    pub fn typed(&self) -> String {
        match self.count {
            Some(count) => format!("{count}{}", write(&self.keys)),
            None => write(&self.keys),
        }
    }

    pub fn clear(&mut self) {
        self.count = None;
        self.keys.clear();
    }
}

/// A table of key sequences.
#[derive(Debug, Clone)]
pub struct Keymap<A> {
    bindings: Vec<(Vec<Press>, A)>,
    counted: bool,
}

impl<A> Default for Keymap<A> {
    fn default() -> Keymap<A> {
        Keymap { bindings: Vec::new(), counted: false }
    }
}

impl<A: Clone> Keymap<A> {
    pub fn new() -> Keymap<A> {
        Keymap::default()
    }

    /// Accepts a leading count, vim's `3j`. Digits then belong to the count rather than to a
    /// binding, except `0` before any other digit — which is why `0` is still free to bind.
    pub fn counted(mut self) -> Keymap<A> {
        self.counted = true;
        self
    }

    /// Binds one sequence. The spec is literal characters plus angle-bracket names:
    /// `<C-k>`, `<S-Tab>`, `<A-x>`, `<CR>`, `<Esc>`, `<Tab>`, `<Space>`, `<BS>`, `<Del>`,
    /// `<Up>`, `<Down>`, `<Left>`, `<Right>`, `<Home>`, `<End>`, `<PageUp>`, `<PageDown>`,
    /// and `<lt>` for a literal `<`.
    ///
    /// A spec that does not parse is dropped, with a debug assertion: a typo in a table should be
    /// caught by the tests that walk it, not turn into a key that silently does nothing else.
    pub fn bind(mut self, spec: &str, action: A) -> Keymap<A> {
        match parse(spec) {
            Some(sequence) if !sequence.is_empty() => self.bindings.push((sequence, action)),
            _ => debug_assert!(false, "keymap: cannot parse {spec:?}"),
        }
        self
    }

    /// Feeds one press through the table.
    pub fn press(&self, pending: &mut Pending, key: &Key, modifiers: Modifiers) -> Resolved<A> {
        let Some(press) = Press::of(key, modifiers) else {
            return Resolved::Ignored;
        };

        // A leading zero is a binding, not a count — `0` is column one in every vi.
        if self.counted
            && pending.keys.is_empty()
            && press.modifiers.is_empty()
            && let Stroke::Char(digit @ '0'..='9') = press.stroke
            && (digit != '0' || pending.count.is_some())
        {
            let value = digit as u32 - '0' as u32;
            let count = pending.count.unwrap_or(0).saturating_mul(10).saturating_add(value);
            pending.count = Some(count.min(u32::MAX / 10));
            return Resolved::Pending;
        }

        pending.keys.push(press);
        if let Some((_, action)) = self.bindings.iter().find(|(sequence, _)| sequence == &pending.keys) {
            let action = action.clone();
            let count = pending.count.unwrap_or(1);
            pending.clear();
            return Resolved::Action(action, count);
        }
        if self.bindings.iter().any(|(sequence, _)| sequence.starts_with(&pending.keys)) {
            return Resolved::Pending;
        }
        pending.clear();
        Resolved::Ignored
    }

    /// Every binding, as it would be written in a help sheet. In the order the table was built,
    /// because the order a table is written in is the order it makes sense in.
    pub fn bindings(&self) -> impl Iterator<Item = (String, &A)> {
        self.bindings.iter().map(|(sequence, action)| (write(sequence), action))
    }

    /// Bindings that are a strict prefix of another, which resolve before the longer one can be
    /// typed. Always empty in a table worth shipping; a test is the place to find out.
    pub fn conflicts(&self) -> Vec<(String, String)> {
        let mut found = Vec::new();
        for (short, _) in &self.bindings {
            for (long, _) in &self.bindings {
                if long.len() > short.len() && long.starts_with(short) {
                    found.push((write(short), write(long)));
                }
            }
        }
        found
    }
}

/// The named keys a spec may use, and what to write them as. The first spelling of a key is the
/// one [`write`] gives back.
const NAMES: &[(&str, Named)] = &[
    ("CR", Named::Enter),
    ("Enter", Named::Enter),
    ("Esc", Named::Escape),
    ("Tab", Named::Tab),
    ("Space", Named::Space),
    ("BS", Named::Backspace),
    ("Del", Named::Delete),
    ("Up", Named::ArrowUp),
    ("Down", Named::ArrowDown),
    ("Left", Named::ArrowLeft),
    ("Right", Named::ArrowRight),
    ("Home", Named::Home),
    ("End", Named::End),
    ("PageUp", Named::PageUp),
    ("PageDown", Named::PageDown),
];

fn parse(spec: &str) -> Option<Vec<Press>> {
    let mut sequence = Vec::new();
    let mut rest = spec;
    while !rest.is_empty() {
        if let Some(tail) = rest.strip_prefix('<') {
            let (inside, after) = tail.split_once('>')?;
            sequence.push(token(inside)?);
            rest = after;
        } else {
            let character = rest.chars().next()?;
            rest = &rest[character.len_utf8()..];
            sequence.push(Press { stroke: Stroke::Char(character), modifiers: Modifiers::empty() });
        }
    }
    Some(sequence)
}

/// The inside of an angle-bracket token: modifier letters, then a key.
fn token(inside: &str) -> Option<Press> {
    if inside == "lt" {
        return Some(Press { stroke: Stroke::Char('<'), modifiers: Modifiers::empty() });
    }
    let mut modifiers = Modifiers::empty();
    let mut rest = inside;
    while let Some((head, tail)) = rest.split_once('-') {
        let modifier = match head {
            "C" => Modifiers::CTRL,
            "S" => Modifiers::SHIFT,
            "A" | "M" => Modifiers::ALT,
            "D" => Modifiers::LOGO,
            _ => break,
        };
        modifiers |= modifier;
        rest = tail;
    }

    if let Some((_, named)) = NAMES.iter().find(|(name, _)| name.eq_ignore_ascii_case(rest)) {
        return Some(Press { stroke: Stroke::Named(*named), modifiers });
    }
    let mut characters = rest.chars();
    let character = characters.next()?;
    if characters.next().is_some() {
        return None;
    }
    // Shift on a character is spent producing the character itself, so `<C-S-k>` is `<C-K>`.
    if modifiers.contains(Modifiers::SHIFT) {
        let upper = character.to_uppercase().next().unwrap_or(character);
        return Some(Press { stroke: Stroke::Char(upper), modifiers: modifiers.difference(Modifiers::SHIFT) });
    }
    Some(Press { stroke: Stroke::Char(character), modifiers })
}

fn write(sequence: &[Press]) -> String {
    let mut out = String::new();
    for press in sequence {
        let named = match press.stroke {
            Stroke::Char('<') => Some("lt"),
            Stroke::Char(_) => None,
            Stroke::Named(named) => NAMES.iter().find(|(_, key)| *key == named).map(|(name, _)| *name),
        };
        if press.modifiers.is_empty()
            && named.is_none()
            && let Stroke::Char(character) = press.stroke
        {
            out.push(character);
            continue;
        }
        out.push('<');
        for (mark, modifier) in [("C-", Modifiers::CTRL), ("A-", Modifiers::ALT), ("D-", Modifiers::LOGO)] {
            if press.modifiers.contains(modifier) {
                out.push_str(mark);
            }
        }
        if press.modifiers.contains(Modifiers::SHIFT) {
            out.push_str("S-");
        }
        match (named, press.stroke) {
            (Some(name), _) => out.push_str(name),
            (None, Stroke::Char(character)) => out.push(character),
            (None, Stroke::Named(_)) => out.push('?'),
        }
        out.push('>');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Action {
        Down,
        Top,
        Bottom,
        Inbox,
        Flagged,
        Palette,
        Column,
        Back,
    }

    fn table() -> Keymap<Action> {
        Keymap::new()
            .counted()
            .bind("j", Action::Down)
            .bind("gg", Action::Top)
            .bind("G", Action::Bottom)
            .bind("gi", Action::Inbox)
            .bind("gf", Action::Flagged)
            .bind("0", Action::Column)
            .bind("<C-k>", Action::Palette)
            .bind("<Esc>", Action::Back)
    }

    fn character(text: &str) -> Key {
        Key::Character(text.into())
    }

    fn press(keys: &Keymap<Action>, pending: &mut Pending, spec: &str) -> Resolved<Action> {
        let sequence = parse(spec).expect("a spec the test wrote");
        let mut last = Resolved::Ignored;
        for step in sequence {
            let key = match step.stroke {
                Stroke::Char(character) => Key::Character(character.to_string().into()),
                Stroke::Named(named) => Key::Named(named),
            };
            last = keys.press(pending, &key, step.modifiers);
        }
        last
    }

    #[test]
    fn a_single_key_fires_on_its_own() {
        let keys = table();
        let mut pending = Pending::default();
        assert_eq!(press(&keys, &mut pending, "j"), Resolved::Action(Action::Down, 1));
        assert!(!pending.is_active(), "nothing is left half-typed");
    }

    #[test]
    fn a_sequence_waits_for_the_rest_of_itself() {
        let keys = table();
        let mut pending = Pending::default();
        assert_eq!(keys.press(&mut pending, &character("g"), Modifiers::empty()), Resolved::Pending);
        assert_eq!(pending.typed(), "g", "an interface can show what is being typed");
        assert_eq!(keys.press(&mut pending, &character("i"), Modifiers::empty()), Resolved::Action(Action::Inbox, 1));
    }

    #[test]
    fn a_sequence_that_goes_nowhere_is_thrown_away_rather_than_kept() {
        let keys = table();
        let mut pending = Pending::default();
        assert_eq!(keys.press(&mut pending, &character("g"), Modifiers::empty()), Resolved::Pending);
        assert_eq!(keys.press(&mut pending, &character("z"), Modifiers::empty()), Resolved::Ignored);
        assert!(!pending.is_active(), "or the next `j` would be read as `gzj`");
        assert_eq!(press(&keys, &mut pending, "j"), Resolved::Action(Action::Down, 1));
    }

    #[test]
    fn a_count_is_typed_in_front_and_arrives_with_the_action() {
        let keys = table();
        let mut pending = Pending::default();
        assert_eq!(press(&keys, &mut pending, "3j"), Resolved::Action(Action::Down, 3));
        assert_eq!(press(&keys, &mut pending, "12j"), Resolved::Action(Action::Down, 12));
        assert_eq!(press(&keys, &mut pending, "10gg"), Resolved::Action(Action::Top, 10));
        // No count is one of the thing, not none of it.
        assert_eq!(press(&keys, &mut pending, "j"), Resolved::Action(Action::Down, 1));
    }

    /// `0` is column one everywhere vi has ever been, so it cannot be the start of a count.
    #[test]
    fn a_leading_zero_is_a_binding_and_a_later_zero_is_a_digit() {
        let keys = table();
        let mut pending = Pending::default();
        assert_eq!(press(&keys, &mut pending, "0"), Resolved::Action(Action::Column, 1));
        assert_eq!(press(&keys, &mut pending, "20j"), Resolved::Action(Action::Down, 20));
    }

    #[test]
    fn a_count_that_leads_nowhere_does_not_stick_to_the_next_key() {
        let keys = table();
        let mut pending = Pending::default();
        assert_eq!(keys.press(&mut pending, &character("4"), Modifiers::empty()), Resolved::Pending);
        assert_eq!(keys.press(&mut pending, &character("z"), Modifiers::empty()), Resolved::Ignored);
        assert_eq!(press(&keys, &mut pending, "j"), Resolved::Action(Action::Down, 1));
    }

    #[test]
    fn shift_is_in_the_character_and_control_is_not() {
        let keys = table();
        let mut pending = Pending::default();
        // The platform reports the shifted character; the binding is written the same way.
        assert_eq!(keys.press(&mut pending, &character("G"), Modifiers::SHIFT), Resolved::Action(Action::Bottom, 1));
        assert_eq!(keys.press(&mut pending, &character("k"), Modifiers::CTRL), Resolved::Action(Action::Palette, 1));
        // ...and the same key without the modifier is not that binding.
        assert_eq!(keys.press(&mut pending, &character("k"), Modifiers::empty()), Resolved::Ignored);
    }

    #[test]
    fn a_named_key_binds_like_any_other() {
        let keys = table();
        let mut pending = Pending::default();
        let escape = Key::Named(Named::Escape);
        assert_eq!(keys.press(&mut pending, &escape, Modifiers::empty()), Resolved::Action(Action::Back, 1));
    }

    #[test]
    fn every_spec_form_round_trips_through_the_way_it_is_written() {
        for spec in ["j", "gg", "<C-k>", "<S-Tab>", "<CR>", "<Esc>", "<A-x>", "<lt>", "g<Space>"] {
            let sequence = parse(spec).unwrap_or_else(|| panic!("{spec} does not parse"));
            assert_eq!(write(&sequence), spec, "{spec}");
        }
        // `<C-S-k>` is the same press as `<C-K>`, and is written the shorter way.
        assert_eq!(parse("<C-S-k>"), parse("<C-K>"));
        // A second spelling parses and is written as the first.
        assert_eq!(write(&parse("<Enter>").unwrap()), "<CR>");
    }

    #[test]
    fn a_table_worth_shipping_has_no_binding_that_is_a_prefix_of_another() {
        assert_eq!(table().conflicts(), Vec::<(String, String)>::new());
        let shadowed = Keymap::new().bind("g", Action::Down).bind("gg", Action::Top);
        assert_eq!(shadowed.conflicts(), vec![("g".to_string(), "gg".to_string())]);
    }

    #[test]
    fn the_table_can_describe_itself_for_a_help_sheet() {
        let written: Vec<String> = table().bindings().map(|(spec, _)| spec).collect();
        assert_eq!(written, ["j", "gg", "G", "gi", "gf", "0", "<C-k>", "<Esc>"]);
    }
}
