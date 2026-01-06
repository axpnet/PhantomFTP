//! TUI Rendering Module
//! 
//! This module handles all the terminal user interface rendering using Ratatui.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Gauge, Wrap},
    Frame,
};
use crate::app::{App, ConnectionDialog};
use crate::ftp::ProtocolType;

// Banner constants (inlined to avoid crate resolution issues)
const PHANTOMFTP_BANNER: &str = r#"
    ██████╗ ██╗  ██╗ █████╗ ███╗   ██╗████████╗ ██████╗ ███╗   ███╗
    ██╔══██╗██║  ██║██╔══██╗████╗  ██║╚══██╔══╝██╔═══██╗████╗ ████║
    ██████╔╝███████║███████║██╔██╗ ██║   ██║   ██║   ██║██╔████╔██║
    ██╔═══╝ ██╔══██║██╔══██║██║╚██╗██║   ██║   ██║   ██║██║╚██╔╝██║
    ██║     ██║  ██║██║  ██║██║ ╚████║   ██║   ╚██████╔╝██║ ╚═╝ ██║
    ╚═╝     ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝   ╚═╝    ╚═════╝ ╚═╝     ╚═╝
                    ███████╗████████╗██████╗ 
                    ██╔════╝╚══██╔══╝██╔══██╗
                    █████╗     ██║   ██████╔╝
                    ██╔══╝     ██║   ██╔═══╝ 
                    ██║        ██║   ██║     
                    ╚═╝        ╚═╝   ╚═╝     
"#;

fn get_banner() -> &'static str {
    PHANTOMFTP_BANNER
}

fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Draws the main UI (header, FTP panels, status bar, connection dialog)
fn draw_main_ui(f: &mut Frame, app: &mut App, size: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Main content
            Constraint::Length(3),  // Status bar
        ])
        .split(size);

    draw_header(f, chunks[0]);
    
    // Split main content into two panels
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);
        
    // Pass mutable reference to app for panel drawing
    draw_local_files_panel(f, app, main_chunks[0]);
    draw_remote_files_panel(f, app, main_chunks[1]);
    
    draw_status_bar(f, app, chunks[2]);
    
    // Draw connection dialog if needed
    if app.show_connection_dialog() {
        let area = f.area();
        let popup_area = Rect {
            x: area.width.saturating_sub(70) / 2,
            y: area.height.saturating_sub(14) / 2,
            width: std::cmp::min(70, area.width),
            height: std::cmp::min(14, area.height),
        };
        // Clear the area behind the popup for opaque background
        f.render_widget(Clear, popup_area);
        draw_connection_dialog(f, &app.connection_dialog, popup_area);
    }
    
    // Draw help dialog if needed
    if app.show_help {
        let area = f.area();
        let popup_area = Rect {
            x: area.width.saturating_sub(75) / 2,
            y: area.height.saturating_sub(26) / 2,
            width: std::cmp::min(75, area.width),
            height: std::cmp::min(26, area.height),
        };
        // Clear the area behind the popup for opaque background
        f.render_widget(Clear, popup_area);
        draw_help_dialog(f, popup_area);
    }
    
    // Draw preview popup if needed
    if app.show_preview {
        draw_preview_popup(f, app);
    }
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();
    
    // If help dialog is active, draw main UI first then overlay help
    if app.show_help {
        draw_main_ui(f, app, size);
        
        // Create a centered popup for help
        let popup_area = centered_rect(70, 80, size);
        // Clear the area behind the popup for opaque background
        f.render_widget(Clear, popup_area);
        draw_help_dialog(f, popup_area);
        return;
    }
    
    // Otherwise just draw the main UI
    draw_main_ui(f, app, size);
}

fn draw_header(f: &mut Frame, area: Rect) {
    let header_text = " 🌐 Rust FTP TUI Client ";
    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));

    f.render_widget(header, area);
}

#[allow(dead_code)]
fn draw_ftp_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Remote files
            Constraint::Percentage(50), // Local files
        ])
        .split(area);

    // Draw remote files panel
    draw_remote_files_panel(f, app, chunks[0]);
    
    // Draw local files panel
    draw_local_files_panel(f, app, chunks[1]);
}

fn draw_remote_files_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let border_style = if app.is_remote_focused() {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };

    // 显示当前远程路径
    let title = format!(" 📁 Remote Files ({}) ", app.current_remote_path());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Show splash banner if not connected
    if !app.is_connected() {
        let banner_text = format!(
            "{}\n\n👻 CyberPunk FTP Client v{}\n\nPress 'c' to connect | 'h' for help | 'q' to quit",
            get_banner(),
            get_version()
        );
        let text = Paragraph::new(banner_text)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        f.render_widget(text, inner_area);
        return;
    }

    // Build file list items
    let items: Vec<ListItem> = app.remote_files.iter().enumerate().map(|(i, file)| {
        let style = if file.is_dir {
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let prefix = if file.is_dir { "📁" } else { "📄" };
        let size = file.size.map(format_file_size).unwrap_or_else(|| "-".to_string());
        
        let content = format!("{} {} ({})", prefix, file.name, size);
        
        let item_style = if Some(i) == app.selected_remote_file {
            style.bg(Color::DarkGray)
        } else {
            style
        };
        
        ListItem::new(content).style(item_style)
    }).collect();

    let list = List::new(items)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray));

    f.render_stateful_widget(list, inner_area, &mut app.remote_list_state);
}

fn draw_local_files_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let border_style = if !app.is_remote_focused() {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray)
    };

    // Show current path in title
    let title = format!(" 💻 {} ", app.current_local_path());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Build file list items from actual local files
    let items: Vec<ListItem> = app.local_files.iter().enumerate().map(|(i, file)| {
        let style = if file.is_dir {
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let prefix = if file.is_dir { "📁" } else { "📄" };
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
    }).collect();

    let list = List::new(items)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray));

    f.render_stateful_widget(list, inner_area, &mut app.local_list_state);
}

static SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
static mut SPINNER_INDEX: usize = 0;

fn get_spinner_char() -> char {
    unsafe {
        let ch = SPINNER_CHARS[SPINNER_INDEX];
        SPINNER_INDEX = (SPINNER_INDEX + 1) % SPINNER_CHARS.len();
        ch
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20),  // Connection status
            Constraint::Min(0),      // Status message or progress
            Constraint::Length(20),  // Help hint or spinner
        ])
        .split(area);

    // Connection status
    let (status, status_color) = if app.is_connected() {
        ("● Connected", Color::Green)
    } else {
        ("○ Disconnected", Color::Red)
    };
    
    let status_text = Paragraph::new(status)
        .style(Style::default().fg(status_color))
        .block(Block::default().borders(Borders::ALL));
    
    f.render_widget(status_text, chunks[0]);

    // Status message or progress
    if app.download_in_progress {
        if let Some(ref progress) = app.transfer_progress {
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL))
                .gauge_style(Style::default().fg(Color::Blue))
                .percent(progress.percentage as u16)
                .label(format!("{}%", progress.percentage));
            
            f.render_widget(gauge, chunks[1]);
        } else {
            let status_msg = Paragraph::new(app.status_message.as_str())
                .style(Style::default().fg(Color::White))
                .block(Block::default().borders(Borders::ALL));
            
            f.render_widget(status_msg, chunks[1]);
        }
    } else if app.upload_in_progress {
        let spinner_char = get_spinner_char();
        let status_with_spinner = format!("{} {}", app.status_message, spinner_char);
        let status_msg = Paragraph::new(status_with_spinner)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL));
        
        f.render_widget(status_msg, chunks[1]);
    } else {
        let status_msg = Paragraph::new(app.status_message.as_str())
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL));
        
        f.render_widget(status_msg, chunks[1]);
    }

    // Help hint or spinner
    if app.upload_in_progress || app.download_in_progress {
        let progress_info = if let Some(ref progress) = app.transfer_progress {
            format!("{} / {} bytes", progress.transferred_bytes, progress.total_bytes)
        } else {
            "In progress...".to_string()
        };
        
        let progress_text = Paragraph::new(progress_info)
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        
        f.render_widget(progress_text, chunks[2]);
    } else {
        let help_text = Paragraph::new("'h' for help")
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        
        f.render_widget(help_text, chunks[2]);
    }
}

fn draw_connection_dialog(f: &mut Frame, dialog: &ConnectionDialog, area: Rect) {
    let block = Block::default()
        .title("Connect to Server")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Create layout for form fields
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Server
            Constraint::Length(1), // Username
            Constraint::Length(1), // Password
            Constraint::Length(1), // Protocol
            Constraint::Length(1), // Empty space
            Constraint::Min(3),    // Instructions (multi-line)
        ])
        .split(inner_area);

    // Protocol display - simplified for FTP/FTPS only
    let protocol_text = match dialog.protocol() {
        ProtocolType::Ftp => "FTP",
        ProtocolType::Ftps => "FTPS (FTP over TLS)",
    };

    // Field labels and values
    let fields = [
        ("Server:", dialog.server()),
        ("Username:", dialog.username()),
        ("Password:", &"*".repeat(dialog.password().len())),
        ("Protocol:", protocol_text),
    ];

    // Render fields
    for (i, (label, value)) in fields.iter().enumerate() {
        let is_selected = i == dialog.selected_field();
        let style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let content = format!("{} {}", label, value);
        let paragraph = Paragraph::new(content)
            .style(style)
            .block(Block::default());

        f.render_widget(paragraph, chunks[i]);
    }

    // Instructions - split on multiple lines for better visibility
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
            Span::styled("↑/↓", Style::default().fg(Color::Yellow)),
            Span::raw(": Select protocol"),
        ]),
    ];
    let instructions = Paragraph::new(instructions_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default());
    
    f.render_widget(instructions, chunks[5]);
}

/// Draw help dialog overlay
fn draw_help_dialog(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::Cyan));

    let help_text = vec![
        Line::from(" NAVIGAZIONE"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tab         ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Switcha tra pannello locale e remoto"),
        ]),
        Line::from(vec![
            Span::styled("Freccia Su/Giù", Style::default().fg(Color::Yellow)),
            Span::raw(" - Naviga tra i file"),
        ]),
        Line::from(vec![
            Span::styled("Enter       ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Entra nella directory selezionata"),
        ]),
        Line::from(vec![
            Span::styled("Backspace   ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Torna alla directory padre"),
        ]),
        Line::from(vec![
            Span::styled("Space       ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Seleziona/deseleziona file per operazioni batch"),
        ]),
        Line::from(""),
        Line::from(" TRASFERIMENTI"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Invio       ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Scarica (pannello remoto) o carica (pannello locale)"),
        ]),
        Line::from(vec![
            Span::styled("p           ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Anteprima file (solo pannello remoto)"),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+C      ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Annulla trasferimento in corso"),
        ]),
        Line::from(""),
        Line::from(" CONNESSIONE"),
        Line::from(""),
        Line::from(vec![
            Span::styled("c           ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Apri dialog connessione"),
        ]),
        Line::from(vec![
            Span::styled("q/Esc       ", Style::default().fg(Color::Yellow)),
            Span::raw(" - Esci dall'applicazione"),
        ]),
        Line::from(""),
        Line::from(" AI ASSISTANTS CHE HANNO CONTRIBUITO:"),
        Line::from(" - Qwen (sviluppo principale)"),
        Line::from(" - Claude Opus 4.5 (implementazione correttiva)"),
        Line::from(" - Gemini 3 Pro (pianificazione iniziale)"),
        Line::from(" - Grok (suggerimenti rapidi)"),
    ];

    let paragraph = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(paragraph, area);
}

/// Helper function to create a centered rectangle
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

/// Format file size in a human readable way
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

/// Draws the file preview popup
fn draw_preview_popup(f: &mut Frame, app: &App) {
    if !app.show_preview {
        return;
    }
    
    let preview_title = if let Some(filename) = &app.preview_file {
        format!(" Preview: {} ", filename)
    } else {
        " Preview ".to_string()
    };
    
    let preview_content = if let Some(content) = &app.preview_content {
        content.clone()
    } else {
        "No content available".to_string()
    };
    
    // Create a centered popup area
    let size = f.area();
    let block = Block::default()
        .title(preview_title)
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Black).fg(Color::White));
    
    // Calculate popup dimensions (max 80% of screen size)
    let popup_width = std::cmp::min(size.width.saturating_sub(4), 80);
    let popup_height = std::cmp::min(size.height.saturating_sub(4), 20);
    
    // Center the popup
    let popup_area = Rect {
        x: (size.width.saturating_sub(popup_width)) / 2,
        y: (size.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };
    
    // Create inner area for content
    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);
    
    // Display content
    let paragraph = Paragraph::new(preview_content)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default())
        .wrap(ratatui::widgets::Wrap { trim: true });
    
    f.render_widget(paragraph, inner_area);
}
