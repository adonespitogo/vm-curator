//! Small text-layout helpers shared across screens.

/// Approximate the number of lines word-wrapping `text` needs at `width`
/// columns (mirrors ratatui's `Wrap { trim: true }` behavior closely enough
/// for plain space-separated text — good enough to size the block that will
/// actually render it, so hint/help text isn't silently clipped on narrow
/// or fixed-width dialogs).
pub fn wrapped_line_count(text: &str, width: u16) -> u16 {
    if width == 0 {
        return 1;
    }
    let width = width as usize;

    let mut lines: u16 = 1;
    let mut current_len = 0usize;
    for word in text.split_whitespace() {
        let word_len = word.chars().count();
        let needed = if current_len == 0 {
            word_len
        } else {
            current_len + 1 + word_len
        };
        if needed > width && current_len > 0 {
            lines += 1;
            current_len = word_len;
        } else {
            current_len = needed;
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_on_one_line_when_wide_enough() {
        assert_eq!(
            wrapped_line_count("[Enter] Launch  [x] Stop  [m] Manage", 200),
            1
        );
    }

    #[test]
    fn wraps_to_multiple_lines_when_narrow() {
        let narrow = wrapped_line_count("[Enter] Launch  [x] Stop  [m] Manage  [c] Create", 20);
        assert!(
            narrow > 1,
            "expected wrapping at width 20, got {narrow} line(s)"
        );
    }

    #[test]
    fn never_below_one() {
        assert_eq!(wrapped_line_count("", 50), 1);
        assert_eq!(wrapped_line_count("hi", 0), 1);
    }

    #[test]
    fn never_splits_a_single_word() {
        assert_eq!(
            wrapped_line_count("supercalifragilisticexpialidocious", 5),
            1
        );
    }
}
