//! A fuzzy-filtered overlay: a query field, and a scored, sorted list under it.
//!
//! [`Picker`] knows nothing about mail, people, commands, or any other domain — only a list of
//! `T`, a query, and which scored match is selected. A caller supplies a `label` closure to score
//! and search by and a `row` closure to draw each match with; [`view`] composes the whole thing as
//! a `stack` over whatever is already on screen, the same way `chrome::frame` layers window
//! controls over content rather than replacing it.
//!
//! Keyboard handling lives with the caller, not here — the same rule `keymap`'s docs give for
//! mode: this widget holds state, the application decides what a press means. A focused
//! [`iced::widget::text_input`] already reports the characters it consumes as captured, so typing
//! into the query field costs nothing extra to wire up; only the few keys the input does not
//! want — up, down, enter, escape — are the caller's to route here.

use crate::fuzzy;
use crate::theme::{self, palette};
use iced::widget::{column, container, mouse_area, opaque, scrollable, space, stack, text, text_input};
use iced::{Border, Color, Element, Length, Padding, Shadow, Vector};

/// Matches beyond this many are not scored for display — a picker is meant to be narrowed by
/// typing, not scrolled through thousands of rows at a time. [`list`] is the widget for a list
/// that is the point; this one is for a list you are about to stop looking at.
pub const MAX_MATCHES: usize = 50;

/// The full candidate list, a query, and which of the *matches* — not which of the items — is
/// selected.
pub struct Picker<T> {
    items: Vec<T>,
    query: String,
    selected: usize,
}

impl<T> Picker<T> {
    pub fn new(items: Vec<T>) -> Picker<T> {
        Picker { items, query: String::new(), selected: 0 }
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    /// Replaces the query. Selection resets to the top: a filtered-down list putting the
    /// selection somewhere unrelated to the new top match would be surprising.
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.selected = 0;
    }

    /// Replaces the candidate list outright — how a caller implements a query prefix that scopes
    /// to a different set of items entirely rather than filtering the current one.
    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.selected = 0;
    }

    /// Indices into the item list, scored against the current query and sorted best match first,
    /// capped at [`MAX_MATCHES`]. An empty query keeps the items in their given order.
    pub fn matches(&self, label: impl Fn(&T) -> &str) -> Vec<usize> {
        if self.query.is_empty() {
            return (0..self.items.len()).take(MAX_MATCHES).collect();
        }
        let mut scored: Vec<(usize, i64)> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| fuzzy::score(&self.query, label(item)).map(|score| (index, score)))
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.truncate(MAX_MATCHES);
        scored.into_iter().map(|(index, _)| index).collect()
    }

    /// Moves the selection by `delta` among however many matches there currently are, wrapping
    /// either direction.
    pub fn move_selection(&mut self, delta: i32, visible: usize) {
        if visible == 0 {
            self.selected = 0;
            return;
        }
        let at = self.selected as i32 + delta;
        self.selected = at.rem_euclid(visible as i32) as usize;
    }

    /// The item the selection currently points at, if any match exists.
    pub fn selected(&self, label: impl Fn(&T) -> &str) -> Option<&T> {
        let matches = self.matches(label);
        matches.get(self.selected).map(|&index| &self.items[index])
    }

    pub fn item(&self, index: usize) -> &T {
        &self.items[index]
    }
}

/// A stable widget id for the query field, so an application can focus it the moment a picker
/// opens.
pub fn query_id() -> iced::advanced::widget::Id {
    iced::advanced::widget::Id::new("noctalia-picker-query")
}

const WIDTH: f32 = 480.0;
const MAX_HEIGHT: f32 = 360.0;

/// Builds the picker as a `stack` over `content`: a click-catching backdrop, and a centered card
/// holding the query field and the scored list. `row` draws one match, told whether it is the
/// current selection; `on_query` and `on_dismiss` are what typing and clicking the backdrop send.
pub fn view<'a, T, M>(
    picker: &'a Picker<T>,
    content: Element<'a, M>,
    label: impl Fn(&T) -> &str + 'a,
    row: impl Fn(&T, bool) -> Element<'a, M> + 'a,
    on_query: impl Fn(String) -> M + 'a,
    on_dismiss: M,
) -> Element<'a, M>
where
    M: Clone + 'a,
{
    let matches = picker.matches(&label);

    let query = text_input("Type a command…", picker.query())
        .id(query_id())
        .on_input(on_query)
        .padding(Padding::from([theme::SPACE_SM, theme::SPACE_MD]))
        .size(theme::FONT_BODY)
        .style(theme::text_input_style);

    let rows: Vec<Element<'a, M>> =
        matches.iter().enumerate().map(|(position, &index)| row(picker.item(index), position == picker.selected)).collect();

    let list: Element<'a, M> = if rows.is_empty() {
        container(text("No matches").color(palette().on_surface_variant).size(theme::FONT_CAPTION))
            .padding(theme::SPACE_MD)
            .into()
    } else {
        scrollable(column(rows)).height(Length::Shrink).style(theme::scrollable_style).into()
    };

    let card = container(column![query, list].spacing(theme::SPACE_SM))
        .width(WIDTH)
        .max_height(MAX_HEIGHT)
        .padding(theme::CARD_PADDING)
        .style(card_style);

    let backdrop =
        mouse_area(container(space()).width(Length::Fill).height(Length::Fill).style(backdrop_style)).on_press(on_dismiss);

    let centered = container(card).width(Length::Fill).height(Length::Fill).center_x(Length::Fill).padding(64);

    stack![content, opaque(backdrop), opaque(centered)].into()
}

fn card_style(_: &iced::Theme) -> container::Style {
    container::Style {
        background: Some(palette().surface_variant.into()),
        border: Border { color: palette().outline, width: theme::BORDER, radius: theme::RADIUS_LG.into() },
        shadow: Shadow { color: Color { a: 0.45, ..Color::BLACK }, offset: Vector::new(0.0, 8.0), blur_radius: 24.0 },
        ..container::Style::default()
    }
}

fn backdrop_style(_: &iced::Theme) -> container::Style {
    container::Style { background: Some(Color { a: 0.5, ..Color::BLACK }.into()), ..container::Style::default() }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A plain `fn` item can't satisfy the `for<'r> Fn(&'r T) -> &'r str` bound `matches`/`selected`
    // want — its own lifetime parameter fixes the return lifetime to one particular call rather
    // than being higher-ranked. A closure literal, passed directly, infers the higher-ranked form
    // instead, so each test builds its own rather than sharing a named helper.

    #[test]
    fn matches_are_scored_and_sorted_best_first() {
        let picker = {
            let mut picker = Picker::new(vec!["gym class", "go to mail", "archive"]);
            picker.set_query("gm".to_string());
            picker
        };
        let matches = picker.matches(|item: &&str| *item);
        assert_eq!(picker.item(matches[0]), &"go to mail", "the word-start match ranks first");
    }

    #[test]
    fn an_empty_query_keeps_every_item_in_its_given_order() {
        let picker = Picker::new(vec!["c", "a", "b"]);
        let matches = picker.matches(|item: &&str| *item);
        assert_eq!(matches, vec![0, 1, 2]);
    }

    #[test]
    fn selection_wraps_both_directions() {
        let mut picker = Picker::new(vec!["a", "b", "c"]);
        picker.move_selection(-1, 3);
        assert_eq!(picker.selected(|item: &&str| *item), Some(&"c"), "moving up from the top wraps to the bottom");
        picker.move_selection(1, 3);
        assert_eq!(picker.selected(|item: &&str| *item), Some(&"a"), "and back");
    }

    #[test]
    fn a_query_that_matches_nothing_selects_nothing() {
        let mut picker = Picker::new(vec!["a", "b"]);
        picker.set_query("zzz".to_string());
        assert_eq!(picker.selected(|item: &&str| *item), None);
    }

    #[test]
    fn setting_items_resets_the_selection() {
        let mut picker = Picker::new(vec!["a", "b", "c"]);
        picker.move_selection(1, 3);
        picker.set_items(vec!["x", "y"]);
        assert_eq!(picker.selected(|item: &&str| *item), Some(&"x"));
    }
}
