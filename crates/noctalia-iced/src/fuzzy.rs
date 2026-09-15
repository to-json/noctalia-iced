//! A small fuzzy-match scorer, in the shape of `fzf`/`skim`'s: every character of the query has
//! to appear in the candidate in order, and the score rewards them landing close together and at
//! the start of a word. It does not need to out-rank `fzf` itself — only to feel like it, over the
//! short command names and subject lines a picker actually searches.
//!
//! ```
//! use noctalia_iced::fuzzy::score;
//!
//! // "gm" prefers the word-start match in "go to mail" over the buried one in "gym class".
//! assert!(score("gm", "go to mail") > score("gm", "gym class"));
//! assert_eq!(score("xyz", "abc"), None);
//! ```

/// The cost of a gap between two matched characters, per character skipped.
const GAP_PENALTY: i64 = 2;
/// What a run of consecutive matched characters earns on top of the lone-character score, per
/// character in the run after the first.
const CONSECUTIVE_BONUS: i64 = 8;
/// Matching right at the start of a word — the very first character, or the one after a
/// non-alphanumeric — on top of whatever the character was already worth. This is the whole
/// reason "gm" prefers "go mail" over "gym": the `m` lands on a word start in one and mid-word in
/// the other.
const WORD_START_BONUS: i64 = 12;

fn is_word_start(candidate: &[char], index: usize) -> bool {
    if index == 0 {
        return true;
    }
    !candidate[index - 1].is_alphanumeric() && candidate[index].is_alphanumeric()
}

/// The best score for typing `query` to reach `candidate`, matched case-insensitively over
/// characters (not just ASCII bytes, so an accented name scores the same way an ASCII one does),
/// or `None` when `query`'s characters don't all appear in `candidate` in order.
///
/// An empty query matches everything at a score of zero, which is what a picker wants to show
/// before anything has been typed.
///
/// Greedy, not a full dynamic-programming search: each query character takes the nearest
/// remaining match rather than searching every placement for the best-scoring one. That is not
/// always optimal, but it is `O(n)`, it is all a query this short ever needs, and a greedy scorer
/// is one a person can predict — which matters more here than squeezing out the last few points.
pub fn score(query: &str, candidate: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }
    let query: Vec<char> = query.to_lowercase().chars().collect();
    let candidate: Vec<char> = candidate.to_lowercase().chars().collect();

    let mut total = 0i64;
    let mut search_from = 0usize;
    let mut previous_match: Option<usize> = None;
    for &q in &query {
        let found = candidate[search_from..].iter().position(|&c| c == q)? + search_from;
        let mut char_score = 1;
        if is_word_start(&candidate, found) {
            char_score += WORD_START_BONUS;
        }
        match previous_match {
            Some(previous) if found == previous + 1 => char_score += CONSECUTIVE_BONUS,
            Some(previous) => total -= GAP_PENALTY * (found - previous - 1) as i64,
            None => {}
        }
        total += char_score;
        previous_match = Some(found);
        search_from = found + 1;
    }
    Some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_query_character_must_appear_in_order() {
        assert_eq!(score("abc", "acb"), None, "b before c in the query, c before b in the candidate");
        assert!(score("abc", "aXbXc").is_some());
    }

    #[test]
    fn an_empty_query_matches_everything_at_zero() {
        assert_eq!(score("", "anything at all"), Some(0));
    }

    #[test]
    fn matching_is_case_insensitive() {
        assert_eq!(score("GM", "go mail"), score("gm", "Go Mail"));
    }

    #[test]
    fn a_word_start_match_beats_a_mid_word_one() {
        assert!(score("gm", "go to mail").unwrap() > score("gm", "gym class").unwrap());
    }

    #[test]
    fn consecutive_characters_score_higher_than_the_same_letters_spread_out() {
        assert!(score("arch", "archive").unwrap() > score("arch", "a rule for chores").unwrap());
    }

    #[test]
    fn an_accented_candidate_is_matched_by_character_not_by_ascii_byte() {
        // "é" is two bytes in UTF-8; byte-indexed matching would either panic or split it across
        // two "characters" that are really one. Matching by `char` sidesteps both: "josé" it is
        // one query character (folded case-insensitively), and "jos" still matches the ASCII
        // prefix that precedes it.
        assert!(score("josé", "José").is_some());
        assert!(score("jos", "José").is_some());
    }
}
