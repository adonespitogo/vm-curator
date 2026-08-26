use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::app::App;
use crate::ui::widgets::{AsciiInfoWidget, VmListWidget, NBSP};

/// Render the main menu screen
pub fn render(app: &App, frame: &mut Frame) {
    let area = frame.area();

    // The help bar wraps onto as many rows as the terminal width demands,
    // so measure it against this frame's width before laying out the screen.
    let content = build_help_content(app);
    let help_height = help_bar_height(app, area.width);

    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),           // Title
            Constraint::Min(10),             // Main content
            Constraint::Length(help_height), // Status/help bar
        ])
        .split(area);

    // Render title
    render_title(app, chunks[0], frame);

    // Split main content: VM list on left, info on right
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    // Render VM list
    VmListWidget::new(app).render(main_chunks[0], frame.buffer_mut());

    // Render ASCII art and info
    let vm_name = app
        .selected_vm()
        .map(|vm| vm.display_name())
        .unwrap_or_else(|| "No VM selected".to_string());

    let os_info = app.selected_vm_info();
    let ascii_art = app.selected_vm_ascii();

    let notes = app.selected_vm().and_then(|vm| vm.notes.as_deref());

    AsciiInfoWidget {
        ascii_art,
        os_info: os_info.as_ref(),
        vm_name: &vm_name,
        scroll: app.info_scroll,
        notes,
    }
    .render(main_chunks[1], frame.buffer_mut());

    // Render help bar
    render_help_bar(content, chunks[2], frame);
}

fn render_title(app: &App, area: Rect, frame: &mut Frame) {
    // Format the library path, shortening home directory to ~
    let library_path = &app.config.vm_library_path;
    let display_path = if let Some(home) = dirs::home_dir() {
        if let Ok(stripped) = library_path.strip_prefix(&home) {
            format!("~/{}", stripped.display())
        } else {
            library_path.display().to_string()
        }
    } else {
        library_path.display().to_string()
    };

    let title = Paragraph::new(vec![Line::from(vec![
        Span::styled(
            " VM Curator ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("(QEMU VM Library in {})", display_path),
            Style::default().fg(Color::Gray),
        ),
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    )
    .alignment(Alignment::Center);

    frame.render_widget(title, area);
}

/// Static (key, label) pairs for the help bar's normal grid of hints, in
/// display order (filled column-major: down each column, then to the next).
const HELP_ITEMS: &[(&str, &str)] = &[
    ("Enter", "Launch"),
    ("x", "Stop"),
    ("m", "Manage"),
    ("c", "Create"),
    ("i", "Import"),
    ("s", "Settings"),
    ("n", "Networks"),
    ("g", "Groups"),
    ("/", "Search"),
    ("?", "Help"),
    ("q", "Quit"),
];

/// Widest most columns the help grid will use, even on very wide terminals.
const MAX_HELP_COLUMNS: usize = 3;

/// The help bar's content: either the normal hint grid, or a single
/// status/stopping-VM message that overrides it. Owned (`'static`) so it can
/// be measured and rendered without borrowing from `app` across the layout
/// split.
enum HelpContent {
    Grid(&'static [(&'static str, &'static str)]),
    Message(Span<'static>),
}

/// Height (including top/bottom borders) the help bar will render at for the
/// given terminal width. Exposed so mouse-click hit testing in `ui::mod` can
/// locate the VM list area without duplicating the grid/wrap layout.
pub fn help_bar_height(app: &App, width: u16) -> u16 {
    let inner_width = width.saturating_sub(2);
    match build_help_content(app) {
        HelpContent::Message(span) => hints_line_count(&[span], inner_width) + 2,
        HelpContent::Grid(items) => {
            let (_, rows) = grid_dimensions(items.len(), max_item_text_width(items), inner_width);
            rows as u16 + 2
        }
    }
}

/// Word-wrapped line count for a list of hint spans (concatenates their text
/// and delegates to the shared wrap estimator).
fn hints_line_count(spans: &[Span], width: u16) -> u16 {
    let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
    crate::ui::widgets::wrapped_line_count(&text, width)
}

/// Width in characters of the widest `"[key] label"` item, used to size grid
/// columns.
fn max_item_text_width(items: &[(&str, &str)]) -> u16 {
    items
        .iter()
        .map(|(key, label)| format!("[{key}] {label}").chars().count() as u16)
        .max()
        .unwrap_or(0)
}

/// Pick a column count that fits `content_width` (never more than
/// `MAX_HELP_COLUMNS`, never fewer than 1) and the resulting row count for
/// `item_count` items filled column-major.
fn grid_dimensions(item_count: usize, max_item_width: u16, content_width: u16) -> (usize, usize) {
    let col_width = max_item_width.saturating_add(2).max(1);
    let cols = (content_width / col_width).clamp(1, MAX_HELP_COLUMNS as u16) as usize;
    let cols = cols.min(item_count.max(1));
    let rows = item_count.div_ceil(cols).max(1);
    (cols, rows)
}

/// Build the help bar's content: the normal hint grid, or a status/stopping-VM
/// message that overrides it.
fn build_help_content(app: &App) -> HelpContent {
    // Add status message if present (overrides everything)
    if let Some(ref msg) = app.status_message {
        return HelpContent::Message(Span::styled(msg.clone(), Style::default().fg(Color::Green)));
    }

    // Show stopping VM status
    if let Some((id, sent_at)) = app.stopping_vms.iter().next() {
        let elapsed = sent_at.elapsed().as_secs();
        let vm_name = app
            .vms
            .iter()
            .find(|vm| &vm.id == id)
            .map(|vm| vm.display_name())
            .unwrap_or_else(|| id.clone());
        let text = if elapsed >= 10 {
            format!("Stopping {}... (press x to force stop)", vm_name)
        } else {
            format!("Stopping {}...", vm_name)
        };
        return HelpContent::Message(Span::styled(text, Style::default().fg(Color::Yellow)));
    }

    HelpContent::Grid(HELP_ITEMS)
}

fn render_help_bar(content: HelpContent, area: Rect, frame: &mut Frame) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    match content {
        HelpContent::Message(span) => {
            let help = Paragraph::new(Line::from(span))
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Center);
            frame.render_widget(help, inner);
        }
        HelpContent::Grid(items) => render_help_grid(items, inner, frame),
    }
}

/// Render hint items in a column-major grid (filled down each column, then
/// to the next), with the column count chosen to fit `area`'s width.
fn render_help_grid(items: &[(&str, &str)], area: Rect, frame: &mut Frame) {
    let (cols, rows) = grid_dimensions(items.len(), max_item_text_width(items), area.width);

    // A 1-cell left margin keeps the first column's hints from butting up
    // against the block border now that columns are left-aligned.
    let area = area.inner(Margin::new(1, 0));
    let col_constraints = vec![Constraint::Ratio(1, cols as u32); cols];
    let col_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(col_constraints)
        .split(area);

    for (col_idx, chunk) in col_chunks.iter().enumerate() {
        let start = col_idx * rows;
        if start >= items.len() {
            continue;
        }
        let end = (start + rows).min(items.len());

        let lines: Vec<Line> = items[start..end]
            .iter()
            .map(|(key, label)| {
                Line::from(vec![
                    Span::styled(format!("[{key}]"), Style::default().fg(Color::Yellow)),
                    Span::raw(format!("{NBSP}{label}")),
                ])
            })
            .collect();

        let para = Paragraph::new(lines).alignment(Alignment::Left);
        frame.render_widget(para, *chunk);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_line_count_concatenates_spans_before_wrapping() {
        let spans = vec![
            Span::raw("[Enter] Launch  "),
            Span::raw("[x] Stop  "),
            Span::raw("[m] Manage"),
        ];
        assert_eq!(hints_line_count(&spans, 200), 1);
        let narrow = hints_line_count(&spans, 10);
        assert!(
            narrow > 1,
            "expected wrapping at width 10, got {narrow} line(s)"
        );
    }
}
