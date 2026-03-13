//! TUI Rendering Module
//!
//! Handles all terminal user interface rendering using Ratatui.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::app::App;
use crate::ftp::ProtocolType;

/// Thread-safe spinner index (replaces `static mut`)
static SPINNER_INDEX: AtomicUsize = AtomicUsize::new(0);
const SPINNER_CHARS: &[char] = &['\u{280B}', '\u{2819}', '\u{2839}', '\u{2838}', '\u{283C}', '\u{2834}', '\u{2826}', '\u{2827}', '\u{2807}', '\u{280F}'];

fn get_spinner_char() -> char {
    let idx = SPINNER_INDEX.fetch_add(1, Ordering::Relaxed) % SPINNER_CHARS.len();
    SPINNER_CHARS[idx]
}

fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ── Main Draw ───────────────────────────────────────────────────

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    draw_main_ui(f, app, size);

    // Overlay help dialog on top if active
    if app.show_help {
        let popup_area = centered_rect(70, 80, size);
        f.render_widget(Clear, popup_area);
        draw_help_dialog(f, popup_area);
    }
}

fn draw_main_ui(f: &mut Frame, app: &mut App, size: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),   // Main content
            Constraint::Length(3), // Status bar
        ])
        .split(size);

    draw_header(f, chunks[0]);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    draw_local_files_panel(f, app, main_chunks[0]);
    draw_remote_files_panel(f, app, main_chunks[1]);
    draw_status_bar(f, app, chunks[2]);

    if app.show_connection_dialog() {
        let area = f.area();
        let popup_area = Rect {
            x: area.width.saturating_sub(70) / 2,
            y: area.height.saturating_sub(14) / 2,
            width: std::cmp::min(70, area.width),
            height: std::cmp::min(14, area.height),
        };
        f.render_widget(Clear, popup_area);
        draw_connection_dialog(f, &app.connection_dialog, popup_area);
    }

    if app.show_preview {
        draw_preview_popup(f, app);
    }
}

// ── Header ──────────────────────────────────────────────────────

fn draw_header(f: &mut Frame, area: Rect) {
    let header = Paragraph::new(format!(" \u{1F47B} PhantomFTP v{} ", get_version()))
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );
    f.render_widget(header, area);
}

// ── File Panels ─────────────────────────────────────────────────

fn draw_local_files_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let border_style = if !app.is_remote_focused() {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };

    let title = format!(" \u{1F4BB} {} ", app.current_local_path());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let items: Vec<ListItem> = app
        .local_files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let style = if file.is_dir {
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if file.is_dir { "\u{1F4C1}" } else { "\u{1F4C4}" };
            let size = file.size.map(format_file_size).unwrap_or_default();

            let content = if size.is_empty() {
                format!("{} {}", prefix, file.name)
            } else {
                format!("{} {} ({})", prefix, file.name, size)
            };

            let item_style = if Some(i) == app.selected_local_file {
                style.bg(Color::DarkGray)
            } else {
                style
            };

            ListItem::new(content).style(item_style)
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .add_modifier(Modifier::BOLD)
            .bg(Color::DarkGray),
    );

    f.render_stateful_widget(list, inner_area, &mut app.local_list_state);
}

fn draw_remote_files_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let border_style = if app.is_remote_focused() {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };

    let title = format!(" \u{1F4C1} Remote Files ({}) ", app.current_remote_path());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Show splash banner if not connected
    if !app.is_connected() {
        let banner_text = format!(
            "{}\n\n\u{1F47B} PhantomFTP v{}\n\nPress 'c' to connect | 'h' for help | 'q' to quit",
            crate::banner::get_banner(),
            get_version()
        );
        let text = Paragraph::new(banner_text)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        f.render_widget(text, inner_area);
        return;
    }

    let items: Vec<ListItem> = app
        .remote_files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let style = if file.is_dir {
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if file.is_dir { "\u{1F4C1}" } else { "\u{1F4C4}" };
            let size = file
                .size
                .map(format_file_size)
                .unwrap_or_else(|| "-".to_string());

            let content = format!("{} {} ({})", prefix, file.name, size);

            let item_style = if Some(i) == app.selected_remote_file {
                style.bg(Color::DarkGray)
            } else {
                style
            };

            ListItem::new(content).style(item_style)
        })
        .collect();

    let list = List::new(items).highlight_style(
        Style::default()
            .add_modifier(Modifier::BOLD)
            .bg(Color::DarkGray),
    );

    f.render_stateful_widget(list, inner_area, &mut app.remote_list_state);
}

// ── Status Bar ──────────────────────────────────────────────────

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20), // Connection status
            Constraint::Min(0),    // Status message or progress
            Constraint::Length(20), // Help hint or transfer info
        ])
        .split(area);

    // Connection status
    let (status, status_color) = if app.is_connected() {
        ("\u{25CF} Connected", Color::Green)
    } else {
        ("\u{25CB} Disconnected", Color::Red)
    };

    let status_text = Paragraph::new(status)
        .style(Style::default().fg(status_color))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(status_text, chunks[0]);

    // Middle: status or progress
    if app.download_in_progress {
        if let Some(ref progress) = app.transfer_progress {
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL))
                .gauge_style(Style::default().fg(Color::Blue))
                .percent(progress.percentage as u16)
                .label(format!("{}%", progress.percentage));
            f.render_widget(gauge, chunks[1]);
        } else {
            render_status_msg(f, &app.status_message, Color::White, chunks[1]);
        }
    } else if app.upload_in_progress {
        let spinner_char = get_spinner_char();
        let text = format!("{} {}", app.status_message, spinner_char);
        render_status_msg(f, &text, Color::Yellow, chunks[1]);
    } else {
        render_status_msg(f, &app.status_message, Color::White, chunks[1]);
    }

    // Right: transfer info or help hint
    if app.upload_in_progress || app.download_in_progress {
        let info = if let Some(ref progress) = app.transfer_progress {
            format!(
                "{} / {} bytes",
                progress.transferred_bytes, progress.total_bytes
            )
        } else {
            "In progress...".to_string()
        };

        let text = Paragraph::new(info)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(text, chunks[2]);
    } else {
        let text = Paragraph::new("'h' for help")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(text, chunks[2]);
    }
}

fn render_status_msg(f: &mut Frame, msg: &str, color: Color, area: Rect) {
    let text = Paragraph::new(msg)
        .style(Style::default().fg(color))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(text, area);
}

// ── Connection Dialog ───────────────────────────────────────────

fn draw_connection_dialog(
    f: &mut Frame,
    dialog: &crate::app::ConnectionDialog,
    area: Rect,
) {
    let block = Block::default()
        .title("Connect to Server")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Server
            Constraint::Length(1), // Username
            Constraint::Length(1), // Password
            Constraint::Length(1), // Protocol
            Constraint::Length(1), // Spacer
            Constraint::Min(3),   // Instructions
        ])
        .split(inner_area);

    let protocol_text = match dialog.protocol() {
        ProtocolType::Ftp => "FTP",
        ProtocolType::Ftps => "FTPS (FTP over TLS)",
    };

    let fields = [
        ("Server:", dialog.server()),
        ("Username:", dialog.username()),
        ("Password:", &"*".repeat(dialog.password().len())),
        ("Protocol:", protocol_text),
    ];

    for (i, (label, value)) in fields.iter().enumerate() {
        let style = if i == dialog.selected_field() {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let paragraph = Paragraph::new(format!("{} {}", label, value))
            .style(style)
            .block(Block::default());
        f.render_widget(paragraph, chunks[i]);
    }

    let instructions_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::raw(": Next field  |  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(": Connect"),
        ]),
        Line::from(vec![
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(": Cancel    |  "),
            Span::styled("\u{2191}/\u{2193}", Style::default().fg(Color::Yellow)),
            Span::raw(": Select protocol"),
        ]),
    ];
    let instructions = Paragraph::new(instructions_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default());
    f.render_widget(instructions, chunks[5]);
}

// ── Help Dialog ─────────────────────────────────────────────────

fn draw_help_dialog(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::Cyan));

    let help_text = vec![
        Line::from(" NAVIGATION"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tab         ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Switch between local and remote panel"),
        ]),
        Line::from(vec![
            Span::styled("Up/Down     ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Navigate file list"),
        ]),
        Line::from(vec![
            Span::styled("Enter       ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Enter selected directory"),
        ]),
        Line::from(vec![
            Span::styled("Backspace   ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Go to parent directory"),
        ]),
        Line::from(""),
        Line::from(" TRANSFERS"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter       ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Download (remote) or Upload (local)"),
        ]),
        Line::from(vec![
            Span::styled("p           ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Preview file (remote panel)"),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+C      ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Cancel active transfer"),
        ]),
        Line::from(""),
        Line::from(" CONNECTION"),
        Line::from(""),
        Line::from(vec![
            Span::styled("c           ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Open connection dialog"),
        ]),
        Line::from(vec![
            Span::styled("r           ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Refresh remote file listing"),
        ]),
        Line::from(vec![
            Span::styled("l           ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Refresh local file listing"),
        ]),
        Line::from(vec![
            Span::styled("q / Esc     ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Quit application"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " PhantomFTP - by axpdev",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::ITALIC),
        )),
    ];

    let paragraph = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

// ── Preview Popup ───────────────────────────────────────────────

fn draw_preview_popup(f: &mut Frame, app: &App) {
    let preview_title = match &app.preview_file {
        Some(filename) => format!(" Preview: {} ", filename),
        None => " Preview ".to_string(),
    };

    let preview_content = match &app.preview_content {
        Some(content) => content.clone(),
        None => "No content available".to_string(),
    };

    let size = f.area();
    let block = Block::default()
        .title(preview_title)
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));

    let popup_width = std::cmp::min(size.width.saturating_sub(4), 80);
    let popup_height = std::cmp::min(size.height.saturating_sub(4), 20);

    let popup_area = Rect {
        x: (size.width.saturating_sub(popup_width)) / 2,
        y: (size.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    f.render_widget(Clear, popup_area);

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let paragraph = Paragraph::new(preview_content)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default())
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, inner_area);
}

// ── Helpers ─────────────────────────────────────────────────────

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}
