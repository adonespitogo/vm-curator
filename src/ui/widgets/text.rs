//! Small text-layout helpers shared across screens.

/// Approximate the number of lines word-wrapping `text` needs at `width`
/// columns (mirrors ratatui's `Wrap { trim: true }` behavior closely enough
/// for plain space-separated text — good enough to size the block that will
/// actually render it, so hint/help text isn't silently clipped on narrow
/// or fixed-width dialogs).
///
/// Preserves the real length of whitespace runs between words (rather than
/// collapsing them to a single space) — hint text is built from spans like
/// `" Launch "` + `" [x]"`, which routinely leaves double spaces between
/// entries, and treating those as single spaces under-counts the width
/// actually needed and under-allocates the container's height.
pub fn wrapped_line_count(text: &str, width: u16) -> u16 {
    if width == 0 {
        return 1;
    }
    let width = width as usize;

    let mut lines: u16 = 1;
    let mut current_len = 0usize;
    let mut chars = text.chars().peekable();

    loop {
        let mut space_len = 0usize;
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            space_len += 1;
            chars.next();
        }

        let mut word_len = 0usize;
        while chars.peek().is_some_and(|c| !c.is_whitespace()) {
            word_len += 1;
            chars.next();
        }
        if word_len == 0 {
            break; // only trailing/no whitespace left
        }

        // Wrap{trim:true} drops the whitespace run right after a wrap point
        // (and any leading the text starts with), so it never counts toward
        // the next line's width.
        let needed = if current_len == 0 {
            word_len
        } else {
            current_len + space_len + word_len
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

    #[test]
    fn double_spaces_between_words_count_toward_line_width() {
        // Regression test: "AAAA" + 2 spaces + "BB" is 8 columns wide. A
        // width of 7 must wrap even though the single-space model
        // (4 + 1 + 2 = 7) would incorrectly say it fits.
        assert_eq!(wrapped_line_count("AAAA  BB", 7), 2);
        assert_eq!(wrapped_line_count("AAAA  BB", 8), 1);
    }

    #[test]
    fn matches_real_help_bar_hint_text_wrap_width() {
        // The actual main-menu help bar hint string (see main_menu.rs),
        // which has double spaces between most entries. At a width that
        // fits everything under the naive single-space model (122) but not
        // the real double-spaced text (134), this must still report 2 lines.
        let hints = " [Enter] Launch  [x] Stop  [m] Manage  [c] Create  [i] Import  \
            [s] Settings  [n] Networks  [g] Groups  [/] Search  [?] Help  [q] Quit ";
        assert_eq!(wrapped_line_count(hints, 128), 2);
    }
}
