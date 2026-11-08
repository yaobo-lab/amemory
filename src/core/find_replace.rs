//! Literal, case-insensitive find/replace for the edit panel's find bar.
//!
//! Ported from Clippi: ASCII queries use `eq_ignore_ascii_case`, other text
//! compares per character with `to_lowercase`. Ranges are byte offsets into
//! the searched text.

use std::ops::Range;

/// Cap on collected matches for one query.
pub const MAX_MATCHES: usize = 10_000;

/// The matches of one query, in order of appearance.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct MatchSet {
    /// Byte ranges of the first [`MAX_MATCHES`] matches.
    pub ranges: Vec<Range<usize>>,
    /// Total number of matches, including those beyond the cap.
    pub total: usize,
}

/// All non-overlapping matches of `query` in `text`.
#[must_use]
pub fn find_matches(text: &str, query: &str) -> MatchSet {
    let mut set = MatchSet::default();
    visit_matches(text, query, |range| {
        set.total += 1;
        if set.ranges.len() < MAX_MATCHES {
            set.ranges.push(range);
        }
    });
    set
}

/// Replace every match. Returns the new text, replacement count and first match offset.
#[must_use]
pub fn replace_all(text: &str, query: &str, replacement: &str) -> (String, usize, Option<usize>) {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    let mut count = 0;
    let mut first = None;
    visit_matches(text, query, |range| {
        first.get_or_insert(range.start);
        out.push_str(&text[at..range.start]);
        out.push_str(replacement);
        at = range.end;
        count += 1;
    });
    out.push_str(&text[at..]);
    (out, count, first)
}

/// Replace only the first match of `query` in `text`. Returns the updated text and whether a match was replaced.
#[must_use]
pub fn replace_first(text: &str, query: &str, replacement: &str) -> (String, bool) {
    let set = find_matches(text, query);
    let Some(first_range) = set.ranges.first() else {
        return (text.to_string(), false);
    };
    let mut out = String::with_capacity(text.len() + replacement.len());
    out.push_str(&text[..first_range.start]);
    out.push_str(replacement);
    out.push_str(&text[first_range.end..]);
    (out, true)
}

/// Index of the first match at or after `offset`, wrapping to `0`.
#[must_use]
pub fn index_at_or_after(ranges: &[Range<usize>], offset: usize) -> usize {
    ranges.iter().position(|r| r.start >= offset).unwrap_or(0)
}

fn find_ascii_ignore_case(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|w| w.eq_ignore_ascii_case(needle))
}

fn visit_matches(text: &str, query: &str, mut visit: impl FnMut(Range<usize>)) {
    if query.is_empty() {
        return;
    }
    if text.is_ascii() && query.is_ascii() {
        let haystack = text.as_bytes();
        let needle = query.as_bytes();
        let mut at = 0;
        while let Some(found) = find_ascii_ignore_case(&haystack[at..], needle) {
            let start = at + found;
            let end = start + needle.len();
            visit(start..end);
            at = end;
        }
        return;
    }
    let mut at = 0;
    while at < text.len() {
        let Some(ch) = text[at..].chars().next() else {
            break;
        };
        if let Some(end) = match_end_at(text, at, query) {
            visit(at..end);
            at = end;
        } else {
            at += ch.len_utf8();
        }
    }
}

fn match_end_at(text: &str, start: usize, query: &str) -> Option<usize> {
    let rest = &text[start..];
    let mut chars = rest.char_indices();
    let mut matched = None;
    for q in query.chars() {
        let (_, t) = chars.next()?;
        if !(t == q || t.to_lowercase().eq(q.to_lowercase())) {
            return None;
        }
        matched = Some(start + rest.len() - chars.as_str().len());
    }
    matched
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_are_literal_and_ignore_case() {
        let r: Vec<_> = find_matches("Hello HELLO hello", "hello").ranges;
        assert_eq!(r, vec![0..5, 6..11, 12..17]);
        assert_eq!(
            find_matches("工作_计划_工作", "_").ranges,
            vec![6..7, 13..14]
        );
    }

    #[test]
    #[allow(clippy::single_range_in_vec_init)]
    fn replace_all_rewrites_matches() {
        assert_eq!(
            replace_all("one_two_three", "_", " "),
            ("one two three".into(), 2, Some(3))
        );
        assert_eq!(index_at_or_after(&[0..1, 4..5], 2), 1);
        let single = [0..1];
        assert_eq!(index_at_or_after(&single, 9), 0);
    }
}
