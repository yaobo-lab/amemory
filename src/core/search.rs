//! Search query tokenization and snippet highlighting calculation.

use std::ops::Range;

use super::find_replace::find_matches;

/// Segment of text with a flag indicating whether it should be highlighted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    /// Text slice.
    pub text: String,
    /// Whether this span matches the query.
    pub is_highlight: bool,
}

/// Split `text` into regular and highlighted spans for `query`.
#[must_use]
pub fn highlight_text(text: &str, query: &str) -> Vec<HighlightSpan> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return vec![HighlightSpan {
            text: text.to_string(),
            is_highlight: false,
        }];
    }

    let match_set = find_matches(text, trimmed);
    if match_set.ranges.is_empty() {
        return vec![HighlightSpan {
            text: text.to_string(),
            is_highlight: false,
        }];
    }

    let mut spans = Vec::new();
    let mut cursor = 0;

    for Range { start, end } in match_set.ranges {
        if start > cursor {
            spans.push(HighlightSpan {
                text: text[cursor..start].to_string(),
                is_highlight: false,
            });
        }
        spans.push(HighlightSpan {
            text: text[start..end].to_string(),
            is_highlight: true,
        });
        cursor = end;
    }

    if cursor < text.len() {
        spans.push(HighlightSpan {
            text: text[cursor..].to_string(),
            is_highlight: false,
        });
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_splits_correctly() {
        let spans = highlight_text("Hello Rust World", "rust");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "Hello ");
        assert!(!spans[0].is_highlight);
        assert_eq!(spans[1].text, "Rust");
        assert!(spans[1].is_highlight);
        assert_eq!(spans[2].text, " World");
        assert!(!spans[2].is_highlight);
    }
}
