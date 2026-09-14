//! A windowed list: fixed-height rows, only the visible ones built.
//!
//! iced's [`scrollable`](iced::widget::scrollable) lays out everything inside it, so a column of
//! fifty thousand rows is fifty thousand widgets measured on every frame. A mail index is that
//! long. The way out is arithmetic rather than a new widget: rows here are a fixed height, so
//! which ones are on screen is division, and the ones that are not become two spacers — one above,
//! one below — that hold the scrollbar at the right size and the rows at the right place.
//!
//! ```
//! use noctalia_iced::list;
//!
//! let pitch = list::pitch(54.0, 1.0);
//! // 50_000 rows, scrolled to 10_000 px, in a 600 px viewport.
//! let window = list::window(50_000, pitch, 10_000.0, 600.0);
//! assert!(window.len() < 30, "a dozen rows and a little slack, not fifty thousand");
//! ```
//!
//! Everything a row does otherwise is unchanged: put [`windowed`] in a `scrollable`, keep the
//! selection bar in a `stack` over it — the bar is positioned against the whole list rather than
//! the window, and [`total`] is the height it has to be.

use iced::widget::{column, space};
use iced::{Element, Length};

/// Rows built above and below the viewport, so a scroll of a few pixels has somewhere to go before
/// the next frame arrives. Four is about a fifth of a screenful at mail-index row heights.
pub const OVERSCAN: usize = 4;

/// Row height plus the hairline between rows: the distance from one row's top to the next one's.
pub fn pitch(row_height: f32, gap: f32) -> f32 {
    row_height + gap
}

/// The height of the whole list, which is what a selection bar and a scroll target measure
/// against. The last row is followed by no gap.
pub fn total(rows: usize, pitch: f32, gap: f32) -> f32 {
    if rows == 0 { 0.0 } else { rows as f32 * pitch - gap }
}

/// Which rows to build, and how much empty space stands in for the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    /// The first row to build.
    pub first: usize,
    /// One past the last row to build.
    pub end: usize,
    /// How many rows there are in total.
    pub rows: usize,
}

impl Window {
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.first)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether this row is one of the built ones.
    pub fn holds(&self, row: usize) -> bool {
        row >= self.first && row < self.end
    }

    /// The rows to build, as an iterator over indices.
    pub fn range(&self) -> std::ops::Range<usize> {
        self.first..self.end
    }
}

/// The rows visible in a viewport `height` tall, scrolled to `offset`, plus [`OVERSCAN`] either
/// side.
///
/// A viewport of zero — which is what a list reports before it has ever been laid out — asks for
/// the top of the list rather than for nothing, so the first frame is not blank.
pub fn window(rows: usize, pitch: f32, offset: f32, height: f32) -> Window {
    if rows == 0 || pitch <= 0.0 {
        return Window { first: 0, end: 0, rows };
    }
    let height = if height > 0.0 { height } else { pitch * (OVERSCAN * 2) as f32 };
    let top = (offset.max(0.0) / pitch).floor() as usize;
    let visible = (height / pitch).ceil() as usize + 1;
    let first = top.saturating_sub(OVERSCAN);
    let end = top.saturating_add(visible).saturating_add(OVERSCAN).min(rows);
    Window { first, end: end.max(first), rows }
}

/// The rows of `window`, with the rest standing in as space.
///
/// `rows` must yield exactly [`Window::len`] elements, in order. The result is `gap`-spaced and
/// exactly [`total`] tall, so a scrollbar and anything stacked over it measure the whole list.
pub fn windowed<'a, M: 'a>(
    window: Window,
    pitch: f32,
    gap: f32,
    rows: impl IntoIterator<Item = Element<'a, M>>,
) -> Element<'a, M> {
    let mut items: Vec<Element<'a, M>> = Vec::with_capacity(window.len() + 2);
    // Each spacer is followed (or preceded) by one of the column's own gaps, so it stands in for
    // the rows it replaces less that gap. A spacer of zero rows is left out entirely rather than
    // pushed at a negative height.
    if window.first > 0 {
        items.push(space().height(window.first as f32 * pitch - gap).into());
    }
    items.extend(rows);
    let after = window.rows.saturating_sub(window.end);
    if after > 0 {
        items.push(space().height(after as f32 * pitch - gap).into());
    }
    column(items).spacing(gap).width(Length::Fill).into()
}

/// Where to scroll so `row` is fully visible, or `None` when it already is.
///
/// The shortest move that does the job: a row above the fold comes to the top, a row below it to
/// the bottom, and a row already on screen does not move the list at all.
pub fn reveal(row: usize, pitch: f32, row_height: f32, offset: f32, viewport: f32) -> Option<f32> {
    if viewport <= 0.0 {
        return None;
    }
    let top = row as f32 * pitch;
    let bottom = top + row_height;
    if top < offset {
        Some(top.max(0.0))
    } else if bottom > offset + viewport {
        Some((bottom - viewport).max(0.0))
    } else {
        None
    }
}

/// How many whole rows fit in the viewport: what `space` and `<C-d>` move by.
pub fn page(pitch: f32, viewport: f32) -> usize {
    if pitch <= 0.0 { 1 } else { ((viewport / pitch).floor() as usize).max(1) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROW: f32 = 54.0;
    const GAP: f32 = 1.0;

    fn p() -> f32 {
        pitch(ROW, GAP)
    }

    /// The whole point: a huge list builds a screenful.
    #[test]
    fn a_long_list_builds_a_screenful_and_not_a_list() {
        let window = window(50_000, p(), 10_000.0, 600.0);
        assert!(window.len() <= 20, "built {} rows for a 600px viewport", window.len());
        assert!(window.holds(10_000 / 55), "the row at the top of the viewport is one of them");
        assert_eq!(window.rows, 50_000);
    }

    /// The spacers have to make the column exactly as tall as the list would have been, or the
    /// scrollbar lies and everything stacked over the rows is in the wrong place.
    #[test]
    fn the_spacers_and_the_gaps_add_up_to_the_height_of_the_whole_list() {
        let pitch = p();
        for (rows, offset, viewport) in [(10, 0.0, 300.0), (10, 200.0, 100.0), (500, 4_000.0, 600.0), (1, 0.0, 600.0)] {
            let window = window(rows, pitch, offset, viewport);
            let built = window.len() as f32;
            // Leading spacer + rows + trailing spacer, and one gap between every pair of children.
            let before = if window.first > 0 { window.first as f32 * pitch - GAP } else { 0.0 };
            let after_rows = rows - window.end;
            let after = if after_rows > 0 { after_rows as f32 * pitch - GAP } else { 0.0 };
            let children = built + u8::from(window.first > 0) as f32 + u8::from(after_rows > 0) as f32;
            let height = before + after + built * ROW + (children - 1.0) * GAP;
            assert!(
                (height - total(rows, pitch, GAP)).abs() < 0.001,
                "{rows} rows at {offset}: column is {height}, list is {}",
                total(rows, pitch, GAP)
            );
        }
    }

    /// And the first built row has to land exactly where the row it stands for would have.
    #[test]
    fn the_first_built_row_lands_where_the_row_it_stands_for_would_have() {
        let pitch = p();
        let window = window(500, pitch, 4_000.0, 600.0);
        assert!(window.first > 0);
        let before = window.first as f32 * pitch - GAP;
        assert!((before + GAP - window.first as f32 * pitch).abs() < 0.001);
    }

    #[test]
    fn an_empty_list_and_a_viewport_of_nothing_are_both_survivable() {
        assert!(window(0, p(), 0.0, 600.0).is_empty());
        assert_eq!(total(0, p(), GAP), 0.0);
        // Before the first layout there is no viewport; show the top rather than nothing.
        let unlaid = window(500, p(), 0.0, 0.0);
        assert!(!unlaid.is_empty(), "the first frame would otherwise be blank");
        assert_eq!(unlaid.first, 0);
    }

    #[test]
    fn the_window_never_runs_past_either_end() {
        let pitch = p();
        // A list shorter than the viewport is built whole.
        assert_eq!(window(6, pitch, 0.0, 600.0), Window { first: 0, end: 6, rows: 6 });
        // A longer one stops at the bottom of the overscan rather than running on.
        assert_eq!(window(20, pitch, 0.0, 600.0).first, 0);
        assert!(window(20, pitch, 0.0, 600.0).end < 20, "rows below the fold are not built");
        let bottom = window(500, pitch, 500.0 * pitch, 600.0);
        assert_eq!(bottom.end, 500);
    }

    #[test]
    fn revealing_a_row_moves_the_list_the_shortest_way_and_not_at_all_when_it_need_not() {
        let pitch = p();
        // On screen already.
        assert_eq!(reveal(3, pitch, ROW, 0.0, 600.0), None);
        // Above the fold: it comes to the top.
        assert_eq!(reveal(2, pitch, ROW, 500.0, 600.0), Some(2.0 * pitch));
        // Below it: it comes to the bottom, which is the shorter move.
        assert_eq!(reveal(20, pitch, ROW, 0.0, 600.0), Some(20.0 * pitch + ROW - 600.0));
        // Row zero cannot ask the list to scroll above its own top.
        assert_eq!(reveal(0, pitch, ROW, 10.0, 600.0), Some(0.0));
    }

    #[test]
    fn a_page_is_whole_rows_and_never_none_of_them() {
        assert_eq!(page(p(), 600.0), 10);
        assert_eq!(page(p(), 10.0), 1, "a viewport shorter than a row still moves by one");
    }
}
