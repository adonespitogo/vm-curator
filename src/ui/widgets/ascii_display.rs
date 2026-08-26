use std::collections::HashMap;
use std::path::PathBuf;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

use crate::metadata::OsInfo;
use crate::vm::qemu_config::NetworkBackend;
use crate::vm::{DiskInfo, QemuConfig};

/// ASCII art and info display widget with scrolling support
pub struct AsciiInfoWidget<'a> {
    pub ascii_art: &'a str,
    pub os_info: Option<&'a OsInfo>,
    pub config: Option<&'a QemuConfig>,
    pub disk_info: &'a HashMap<PathBuf, Option<DiskInfo>>,
    pub vm_name: &'a str,
    pub scroll: u16,
    pub notes: Option<&'a str>,
}

impl<'a> AsciiInfoWidget<'a> {
    pub fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the area first to prevent stale characters when content changes
        Clear.render(area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        block.render(area, buf);

        // Add horizontal padding for elegant margins
        let padded = Rect {
            x: inner.x.saturating_add(2),
            y: inner.y.saturating_add(1),
            width: inner.width.saturating_sub(4),
            height: inner.height.saturating_sub(1),
        };

        // Build the full content as a single scrollable text
        let mut lines: Vec<Line> = Vec::new();

        // ASCII art - preserve exact spacing (no trimming)
        for line in self.ascii_art.trim_start_matches('\n').lines() {
            lines.push(Line::styled(line, Style::default().fg(Color::Green)));
        }
        lines.push(Line::from(""));

        // VM name header
        lines.push(Line::from(Span::styled(
            self.vm_name,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )));

        // OS identity, if known (kept brief — details now live in the config section)
        if let Some(info) = self.os_info {
            lines.push(Line::from(vec![
                Span::styled(&info.name, Style::default().fg(Color::Gray)),
                Span::raw(" | "),
                Span::styled(&info.publisher, Style::default().fg(Color::Gray)),
                Span::raw(" | "),
                Span::styled(&info.release_date, Style::default().fg(Color::Gray)),
            ]));
        }
        lines.push(Line::from(""));

        // Rich VM configuration: CPU, architecture, RAM, disks, network topology
        if let Some(config) = self.config {
            push_config_lines(&mut lines, config, self.disk_info);
        }

        // User notes
        if let Some(notes) = self.notes {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Notes",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            for line in notes.lines() {
                lines.push(Line::from(line.to_string()));
            }
        }

        // Don't use trim: true as it breaks ASCII art spacing
        let para = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));
        para.render(padded, buf);
    }
}

/// Append CPU, architecture, memory, disk, and network-topology details for
/// `config` to `lines`, matching the field set shown by the Configuration
/// screen but with full disk paths and per-NIC topology spelled out.
fn push_config_lines(
    lines: &mut Vec<Line>,
    config: &QemuConfig,
    disk_info: &HashMap<PathBuf, Option<DiskInfo>>,
) {
    lines.push(Line::from(Span::styled(
        "Hardware",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(vec![
        Span::styled("Architecture: ", Style::default().fg(Color::Gray)),
        Span::raw(format!(
            "{} ({})",
            config.emulator.architecture(),
            config.emulator.command()
        )),
    ]));
    let cpu = match &config.cpu_model {
        Some(model) => format!("{} cores ({})", config.cpu_cores, model),
        None => format!("{} cores", config.cpu_cores),
    };
    lines.push(Line::from(vec![
        Span::styled("CPU: ", Style::default().fg(Color::Gray)),
        Span::raw(cpu),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Memory: ", Style::default().fg(Color::Gray)),
        Span::raw(format_memory(config.memory_mb)),
    ]));
    if let Some(machine) = &config.machine {
        lines.push(Line::from(vec![
            Span::styled("Machine: ", Style::default().fg(Color::Gray)),
            Span::raw(machine.clone()),
        ]));
    }
    let mut features = Vec::new();
    if config.enable_kvm {
        features.push("KVM");
    }
    if config.uefi {
        features.push("UEFI");
    }
    if config.tpm {
        features.push("TPM");
    }
    if config.has_gl_acceleration() {
        features.push("3D Accel");
    }
    lines.push(Line::from(vec![
        Span::styled("Features: ", Style::default().fg(Color::Gray)),
        Span::raw(if features.is_empty() {
            "None".to_string()
        } else {
            features.join(", ")
        }),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Disks",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    if config.disks.is_empty() {
        lines.push(Line::styled(
            "  (none)",
            Style::default().fg(Color::DarkGray),
        ));
    }
    for disk in &config.disks {
        let file_name = disk
            .path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_else(|| disk.path.to_string_lossy());
        let mut line = format!(
            "  {} — {:?}, {} [{:?}]",
            file_name, disk.format, disk.interface, disk.role
        );
        if let Some(Some(info)) = disk_info.get(&disk.path) {
            line.push_str(&format!(
                " ({} used / {} capacity)",
                info.disk_size, info.virtual_size
            ));
        }
        lines.push(Line::from(line));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Network",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )));
    if config.networks.is_empty() {
        lines.push(Line::styled(
            "  (none)",
            Style::default().fg(Color::DarkGray),
        ));
    }
    let multi_nic = config.networks.len() > 1;
    for (idx, net) in config.networks.iter().enumerate() {
        let backend_str = match &net.backend {
            NetworkBackend::User => "user/SLIRP (NAT)".to_string(),
            NetworkBackend::Passt => "passt".to_string(),
            NetworkBackend::Bridge(name) => format!("bridge: {}", name),
            NetworkBackend::None => "none".to_string(),
        };
        let label = if multi_nic {
            format!("  NIC {}: ", idx + 1)
        } else {
            "  NIC: ".to_string()
        };
        let mut line = format!("{}{} — {}", label, net.model, backend_str);
        if let Some(mac) = &net.mac_address {
            line.push_str(&format!(" (MAC {})", mac));
        }
        lines.push(Line::from(line));
        for pf in &net.port_forwards {
            lines.push(Line::styled(
                format!("    {} {} -> {}", pf.protocol, pf.host_port, pf.guest_port),
                Style::default().fg(Color::DarkGray),
            ));
        }
    }
}

/// Format memory in MB, with a GB conversion alongside for readability.
fn format_memory(mb: u32) -> String {
    if mb >= 1024 {
        format!("{} MB ({:.1} GB)", mb, mb as f64 / 1024.0)
    } else {
        format!("{} MB", mb)
    }
}

/// Detailed info display (for the info screen)
pub struct DetailedInfoWidget<'a> {
    pub os_info: Option<&'a OsInfo>,
    pub vm_name: &'a str,
}

impl<'a> DetailedInfoWidget<'a> {
    pub fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title(format!(" {} - Details ", self.vm_name))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        block.render(area, buf);

        if let Some(info) = self.os_info {
            let mut text = vec![
                Line::from(vec![
                    Span::styled("Name: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&info.name),
                ]),
                Line::from(vec![
                    Span::styled("Publisher: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&info.publisher),
                ]),
                Line::from(vec![
                    Span::styled("Released: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&info.release_date),
                ]),
                Line::from(vec![
                    Span::styled("Architecture: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&info.architecture),
                ]),
                Line::from(""),
            ];

            // Add long description
            if !info.blurb.long.is_empty() {
                text.push(Line::from(Span::styled(
                    "About",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
                for line in info.blurb.long.lines() {
                    text.push(Line::from(line.to_string()));
                }
                text.push(Line::from(""));
            }

            // Add fun facts
            if !info.fun_facts.is_empty() {
                text.push(Line::from(Span::styled(
                    "Fun Facts",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
                for fact in &info.fun_facts {
                    text.push(Line::from(format!("• {}", fact)));
                }
            }

            let para = Paragraph::new(text).wrap(Wrap { trim: true });
            para.render(inner, buf);
        } else {
            let text = Paragraph::new("No detailed information available for this VM.")
                .style(Style::default().fg(Color::Gray));
            text.render(inner, buf);
        }
    }
}
