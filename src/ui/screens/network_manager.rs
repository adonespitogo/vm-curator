//! Virtual Network Manager screen (issue #53).
//!
//! Lists managed networks with live Active/Inactive status, edits their
//! definitions, and starts/stops them by running the generated
//! `net-up.sh`/`net-down.sh` scripts with elevated privileges (pkexec if
//! available, else sudo in a terminal window) — the TUI itself never
//! modifies host networking.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::*,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::app::{App, ConfirmAction, Screen, UnsavedKind, VNetEditorState};
use crate::vnet::{self, VirtualNetwork};

pub fn render(app: &App, frame: &mut Frame) {
    let area = frame.area();
    let dialog_width = 78.min(area.width.saturating_sub(4));
    let dialog_height = 24.min(area.height.saturating_sub(4));
    let dialog_area = centered_rect(dialog_width, dialog_height, area);
    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(format!(" Networks ({} defined) ", app.vnet_networks.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(inner);

    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header/intro
            Constraint::Length(1), // Spacer
            Constraint::Min(4),    // Network list
            Constraint::Length(2), // Help
            Constraint::Length(1), // Spacer
            Constraint::Length(4), // Notes
        ])
        .split(h_chunks[1]);

    let intro = Paragraph::new("Managed virtual networks (Linux bridges owned by vm-curator).")
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(intro, v_chunks[0]);

    if app.vnet_networks.is_empty() {
        let empty = Paragraph::new("No networks defined yet. Press [c] to create one.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(empty, v_chunks[2]);
    } else {
        let items: Vec<ListItem> = app
            .vnet_networks
            .iter()
            .enumerate()
            .map(|(i, net)| {
                let selected = i == app.vnet_selected;
                let name_style = if selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let active = net.is_active();
                let (status, status_style) = if active {
                    ("● Active  ", Style::default().fg(Color::Green))
                } else {
                    ("○ Inactive", Style::default().fg(Color::DarkGray))
                };
                let dhcp = if net.dhcp { "DHCP" } else { "    " };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{:<13}", net.name), name_style),
                    Span::styled(
                        format!("{:<9}", net.kind.label()),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::styled(format!("{:<19}", net.subnet), name_style),
                    Span::styled(format!("{:<5}", dhcp), Style::default().fg(Color::DarkGray)),
                    Span::styled(status, status_style),
                ]))
            })
            .collect();

        let mut state = ListState::default();
        state.select(Some(app.vnet_selected));
        let list = List::new(items).highlight_symbol("> ");
        frame.render_stateful_widget(list, v_chunks[2], &mut state);
    }

    let help = Paragraph::new(
        "[s] Start  [x] Stop  [a] Start All  [c] Create  [e] Edit  [d] Delete\n\
         [Enter] Toggle  [r] Refresh  [Esc] Back",
    )
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center);
    frame.render_widget(help, v_chunks[3]);

    let notes = Paragraph::new(
        "Start/stop runs the network's generated net-up.sh / net-down.sh with\n\
         pkexec (or sudo in a terminal if pkexec is unavailable) — inspect them\n\
         under ~/.config/vm-curator/networks/. Attach VMs via Network Settings → bridge.",
    )
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(notes, v_chunks[5]);

    // Create/edit form overlays the list
    if let Some(editor) = &app.vnet_editor {
        render_editor(editor, frame);
    }
}

fn render_editor(editor: &VNetEditorState, frame: &mut Frame) {
    let area = frame.area();
    let dialog_width = 56.min(area.width.saturating_sub(6));
    let dialog_height = 15.min(area.height.saturating_sub(6));
    let dialog_area = centered_rect(dialog_width, dialog_height, area);
    frame.render_widget(Clear, dialog_area);

    let title = if editor.original_name.is_some() {
        " Edit Network "
    } else {
        " Create Network "
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .style(Style::default().bg(Color::Black));
    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Name
            Constraint::Length(1), // Type
            Constraint::Length(1), // Subnet
            Constraint::Length(1), // DHCP
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Info / error
            Constraint::Length(2), // Help
        ])
        .split(inner);

    let field = |label: &str, value: String, focus: usize| -> Line<'static> {
        let focused = editor.field_focus == focus;
        let editing_this = focused && editor.editing;
        let shown = if editing_this {
            format!("{}_", editor.edit_buffer)
        } else {
            value
        };
        Line::from(vec![
            Span::styled(
                if focused { "> " } else { "  " }.to_string(),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(format!("{:<9}", label), Style::default().fg(Color::Yellow)),
            Span::styled(
                shown,
                if editing_this {
                    Style::default().fg(Color::Cyan)
                } else if focused {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            ),
        ])
    };

    let name_value = if editor.original_name.is_some() {
        format!("{} (fixed)", editor.name)
    } else {
        editor.name.clone()
    };
    frame.render_widget(Paragraph::new(field("Name:", name_value, 0)), chunks[0]);
    frame.render_widget(
        Paragraph::new(field(
            "Type:",
            format!("[ {} ]  (←/→ toggle)", editor.kind.label()),
            1,
        )),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(field("Subnet:", editor.subnet.clone(), 2)),
        chunks[2],
    );
    frame.render_widget(
        Paragraph::new(field(
            "DHCP:",
            format!("[{}]", if editor.dhcp { "on" } else { "off" }),
            3,
        )),
        chunks[3],
    );

    let info = if let Some(err) = &editor.error {
        Paragraph::new(err.clone()).style(Style::default().fg(Color::Red))
    } else {
        Paragraph::new(
            "Gateway and DHCP range are derived from the subnet.\n\
             NAT = outbound internet; Isolated = host-only.",
        )
        .style(Style::default().fg(Color::DarkGray))
    };
    frame.render_widget(info, chunks[5]);

    let help_text = if editor.editing {
        "[Enter] Done  [Esc] Cancel edit"
    } else {
        "[j/k] Field  [Enter/Tab] Edit field  [←/→] Toggle  [s] Save  [Esc] Cancel"
    };
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    frame.render_widget(help, chunks[6]);
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> Result<()> {
    if app.vnet_editor.is_some() {
        return handle_editor_key(app, key);
    }

    match key.code {
        KeyCode::Esc => {
            app.pop_screen();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.vnet_networks.is_empty() && app.vnet_selected + 1 < app.vnet_networks.len() {
                app.vnet_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.vnet_selected > 0 {
                app.vnet_selected -= 1;
            }
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.vnet_editor = Some(VNetEditorState::new_network());
        }
        KeyCode::Char('e') | KeyCode::Char('E') => {
            if let Some(net) = app.vnet_networks.get(app.vnet_selected) {
                if net.is_active() {
                    app.set_status(
                        "Stop the network before editing (its scripts must match host state)",
                    );
                } else {
                    app.vnet_editor = Some(VNetEditorState::edit(net));
                }
            }
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if let Some(net) = app.vnet_networks.get(app.vnet_selected) {
                if net.is_active() {
                    app.set_status("Stop the network before deleting it");
                } else {
                    app.push_screen(Screen::Confirm(ConfirmAction::DeleteNetwork));
                }
            }
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.reload_vnet_networks();
            app.set_status("Refreshed networks");
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if let Some(net) = app.vnet_networks.get(app.vnet_selected).cloned() {
                start_or_stop(app, &net);
            }
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            start_selected(app);
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            stop_selected(app);
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            start_all(app);
        }
        _ => {}
    }
    Ok(())
}

fn handle_editor_key(app: &mut App, key: KeyEvent) -> Result<()> {
    let in_text_edit = app.vnet_editor.as_ref().map(|e| e.editing).unwrap_or(false);

    if in_text_edit {
        if let Some(editor) = app.vnet_editor.as_mut() {
            match key.code {
                KeyCode::Esc => {
                    editor.editing = false;
                    editor.edit_buffer.clear();
                }
                KeyCode::Enter | KeyCode::Tab => {
                    let value = editor.edit_buffer.trim().to_string();
                    match editor.field_focus {
                        0 => editor.name = value,
                        2 => editor.subnet = value,
                        _ => {}
                    }
                    editor.editing = false;
                    editor.edit_buffer.clear();
                }
                KeyCode::Backspace => {
                    editor.edit_buffer.pop();
                }
                KeyCode::Char(c) if !c.is_whitespace() => {
                    editor.edit_buffer.push(c);
                }
                _ => {}
            }
        }
        return Ok(());
    }

    // App-level actions first (they need `app` unborrowed)
    match key.code {
        KeyCode::Esc => {
            // Prompt before discarding if this session actually changed
            // anything; otherwise there's nothing to lose, so just leave.
            let dirty = app.vnet_editor.as_ref().map(|e| e.dirty()).unwrap_or(false);
            if dirty {
                app.push_screen(Screen::Confirm(ConfirmAction::UnsavedChanges(
                    UnsavedKind::NetworkEdit,
                )));
            } else {
                discard_editor(app);
            }
            return Ok(());
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            save_editor(app);
            return Ok(());
        }
        _ => {}
    }

    let Some(editor) = app.vnet_editor.as_mut() else {
        return Ok(());
    };
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            if editor.field_focus < 3 {
                editor.field_focus += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if editor.field_focus > 0 {
                editor.field_focus -= 1;
            }
        }
        KeyCode::Left | KeyCode::Right => match editor.field_focus {
            1 => editor.kind = editor.kind.toggle(),
            3 => editor.dhcp = !editor.dhcp,
            _ => {}
        },
        KeyCode::Enter | KeyCode::Tab => match editor.field_focus {
            0 if editor.original_name.is_none() => {
                editor.editing = true;
                editor.edit_buffer = editor.name.clone();
            }
            2 => {
                editor.editing = true;
                editor.edit_buffer = editor.subnet.clone();
            }
            1 => editor.kind = editor.kind.toggle(),
            3 => editor.dhcp = !editor.dhcp,
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

/// Discard the create/edit form's in-progress edits and close it without
/// saving. Used by a no-op Esc (nothing changed) and by the "Discard"
/// choice on the confirmation prompt (`ConfirmAction::UnsavedChanges(NetworkEdit)`).
pub(crate) fn discard_editor(app: &mut App) {
    app.vnet_editor = None;
}

/// Build the network from the form, save it, and close the editor. Used by
/// the editor's `[s] Save` key and by the "Save" choice on the same
/// confirmation prompt.
pub(crate) fn save_editor(app: &mut App) {
    let Some(editor) = app.vnet_editor.as_mut() else {
        return;
    };

    // Creating: reject duplicate names
    if editor.original_name.is_none() && app.vnet_networks.iter().any(|n| n.name == editor.name) {
        editor.error = Some(format!("A network named '{}' already exists", editor.name));
        return;
    }

    let built =
        VirtualNetwork::with_defaults(&editor.name, editor.kind, &editor.subnet).map(|mut net| {
            net.dhcp = editor.dhcp && net.dhcp;
            net
        });
    match built {
        Ok(net) => {
            if let Err(e) = vnet::save_network(&vnet::networks_dir(), &net) {
                editor.error = Some(format!("Save failed: {e}"));
                return;
            }
            let name = net.name.clone();
            app.vnet_editor = None;
            app.reload_vnet_networks();
            if let Some(pos) = app.vnet_networks.iter().position(|n| n.name == name) {
                app.vnet_selected = pos;
            }
            app.set_status(format!("Saved network '{name}'"));
        }
        Err(e) => {
            editor.error = Some(e.to_string());
        }
    }
}

/// Run the appropriate script with elevated privileges.
fn start_or_stop(app: &mut App, net: &VirtualNetwork) {
    let start = !net.is_active();
    let action = if start { "Starting" } else { "Stopping" };
    match spawn_network_script(net, start) {
        Ok(()) => app.set_status(format!(
            "{action} '{}' — authorize when prompted, then press r to refresh",
            net.name
        )),
        Err(e) => app.set_status(e),
    }
}

/// Start the selected network if it isn't already active.
fn start_selected(app: &mut App) {
    let Some(net) = app.vnet_networks.get(app.vnet_selected).cloned() else {
        return;
    };
    if net.is_active() {
        app.set_status(format!("'{}' is already active", net.name));
        return;
    }
    match spawn_network_script(&net, true) {
        Ok(()) => app.set_status(format!(
            "Starting '{}' — authorize when prompted, then press r to refresh",
            net.name
        )),
        Err(e) => app.set_status(e),
    }
}

/// Stop the selected network if it isn't already inactive.
fn stop_selected(app: &mut App) {
    let Some(net) = app.vnet_networks.get(app.vnet_selected).cloned() else {
        return;
    };
    if !net.is_active() {
        app.set_status(format!("'{}' is already inactive", net.name));
        return;
    }
    match spawn_network_script(&net, false) {
        Ok(()) => app.set_status(format!(
            "Stopping '{}' — authorize when prompted, then press r to refresh",
            net.name
        )),
        Err(e) => app.set_status(e),
    }
}

/// Start every network that isn't already active.
fn start_all(app: &mut App) {
    let inactive: Vec<VirtualNetwork> = app
        .vnet_networks
        .iter()
        .filter(|n| !n.is_active())
        .cloned()
        .collect();
    if inactive.is_empty() {
        app.set_status("All networks are already active");
        return;
    }

    let mut scripts = Vec::new();
    let mut missing = Vec::new();
    for net in &inactive {
        let script = vnet::script_path(&vnet::networks_dir(), &net.name, true);
        if script.exists() {
            scripts.push(script);
        } else {
            missing.push(net.name.clone());
        }
    }

    if scripts.is_empty() {
        app.set_status(format!(
            "No start scripts found for: {}",
            missing.join(", ")
        ));
        return;
    }

    let count = scripts.len();
    match run_privileged(&scripts) {
        Ok(()) => {
            let mut msg = format!(
                "Starting {count} network(s) — authorize when prompted, then press r to refresh"
            );
            if !missing.is_empty() {
                msg.push_str(&format!("; script missing for: {}", missing.join(", ")));
            }
            app.set_status(msg);
        }
        Err(e) => app.set_status(e),
    }
}

/// Resolve and run the start/stop script for a network with elevated privileges.
fn spawn_network_script(net: &VirtualNetwork, start: bool) -> std::result::Result<(), String> {
    let script = vnet::script_path(&vnet::networks_dir(), &net.name, start);
    if !script.exists() {
        return Err(format!("Script missing: {}", script.display()));
    }
    run_privileged(std::slice::from_ref(&script))
}

/// Run one or more scripts as root in a single privileged invocation (so
/// e.g. "Start All" only prompts for authorization once): prefer `pkexec`
/// (graphical polkit prompt, no terminal required), falling back to `sudo`
/// in a terminal emulator when pkexec isn't installed.
fn run_privileged(scripts: &[std::path::PathBuf]) -> std::result::Result<(), String> {
    let combined = scripts
        .iter()
        .map(|p| shell_quote(&p.to_string_lossy()))
        .collect::<Vec<_>>()
        .join("; ");

    if command_exists("pkexec") {
        use std::process::Stdio;
        // The scripts' own stdout/stderr must not inherit ours: we have no
        // terminal window here, so it would otherwise print straight onto
        // the raw terminal ratatui is drawing to and corrupt the TUI.
        return std::process::Command::new("pkexec")
            .arg("sh")
            .arg("-c")
            .arg(&combined)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Failed to launch pkexec: {e}"));
    }
    run_shell_in_terminal(&combined)
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run `sudo sh -c '<shell_cmd>'` in the first available terminal emulator
/// (same launcher list as the single-GPU passthrough setup).
fn run_shell_in_terminal(shell_cmd: &str) -> std::result::Result<(), String> {
    let terminals: &[(&str, &[&str])] = &[
        ("alacritty", &["-e", "sudo"]),
        ("kitty", &["sudo"]),
        ("ghostty", &["-e", "sudo"]),
        ("gnome-terminal", &["--", "sudo"]),
        ("konsole", &["-e", "sudo"]),
        ("xfce4-terminal", &["-x", "sudo"]),
        ("xterm", &["-e", "sudo"]),
    ];

    for (term, args) in terminals {
        if !command_exists(term) {
            continue;
        }
        let mut cmd = std::process::Command::new(term);
        cmd.args(*args).arg("sh").arg("-c").arg(shell_cmd);
        if cmd.spawn().is_ok() {
            return Ok(());
        }
    }
    Err(
        "No pkexec or terminal found. Install polkit (pkexec), or alacritty, kitty, ghostty, \
         konsole, gnome-terminal — or run the script manually with sudo."
            .to_string(),
    )
}

/// Delete the selected network (called from the confirm dialog).
pub(crate) fn delete_selected(app: &mut App) {
    let Some(net) = app.vnet_networks.get(app.vnet_selected).cloned() else {
        return;
    };
    if net.is_active() {
        app.set_status("Network became active; stop it before deleting");
        return;
    }
    match vnet::delete_network(&vnet::networks_dir(), &net.name) {
        Ok(()) => {
            app.reload_vnet_networks();
            app.set_status(format!("Deleted network '{}'", net.name));
        }
        Err(e) => app.set_status(format!("Delete failed: {e}")),
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

#[cfg(test)]
mod tests {
    use super::shell_quote;

    #[test]
    fn shell_quote_wraps_plain_paths_in_single_quotes() {
        assert_eq!(
            shell_quote("/home/user/.config/vm-curator/networks/net0/net-up.sh"),
            "'/home/user/.config/vm-curator/networks/net0/net-up.sh'"
        );
    }

    #[test]
    fn shell_quote_escapes_embedded_single_quotes() {
        assert_eq!(shell_quote("it's/a/path"), "'it'\\''s/a/path'");
    }

    #[test]
    fn start_all_combines_multiple_scripts_into_one_shell_command() {
        // Mirrors what `run_privileged` builds so "Start All" issues a
        // single pkexec/sudo prompt instead of one per network.
        let scripts = ["/net0/net-up.sh", "/net1/net-up.sh"];
        let combined = scripts
            .iter()
            .map(|p| shell_quote(p))
            .collect::<Vec<_>>()
            .join("; ");
        assert_eq!(combined, "'/net0/net-up.sh'; '/net1/net-up.sh'");
    }
}
