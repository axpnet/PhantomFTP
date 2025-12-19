//! TUI Rendering Module
//! 
//! This module handles all the terminal user interface rendering using Ratatui.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Clear},
    Frame,
};
use crate::app::App;

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Main content
            Constraint::Length(3),  // Status bar
        ])
        .split(size);

    // Draw header
    draw_header(f, chunks[0]);

    // Draw main content
    draw_ftp_panel(f, app, chunks[1]);

    // Draw status bar
    draw_status_bar(f, app, chunks[2]);

    // Draw modal dialogs if any
    if app.show_connection_dialog() {
        draw_connection_dialog(f, app);
    }
}

fn draw_header(f: &mut Frame, area: Rect) {
    let header_text = " 🌐 Rust FTP TUI Client ";
    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)));

    f.render_widget(header, area);
}

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

    let block = Block::default()
        .title(" 📁 Remote Files ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Show connection status if not connected
    if !app.is_connected() {
        let text = Paragraph::new("Press 'c' to connect to FTP server")
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center);
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
        let size = file.size.map(|s| format_file_size(s)).unwrap_or_else(|| "-".to_string());
        
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
        let size = file.size.map(|s| format_file_size(s)).unwrap_or_else(|| "".to_string());
        
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

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(20),  // Connection status
            Constraint::Min(0),      // Status message
            Constraint::Length(20),  // Help hint
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

    // Status message
    let status_msg = Paragraph::new(app.status_message.as_str())
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL));
    
    f.render_widget(status_msg, chunks[1]);

    // Help hint
    let help_text = Paragraph::new("'h' for help")
        .style(Style::default().fg(Color::Gray))
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    
    f.render_widget(help_text, chunks[2]);
}

fn draw_connection_dialog(f: &mut Frame, app: &mut App) {
    let area = f.area();
    
    // Create centered dialog
    let popup_area = centered_rect(60, 40, area);
    
    // Clear the area behind the popup
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" 🔌 Connect to FTP Server ")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray))
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);
    
    // Create form layout
    let form_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .split(inner_area);

    // Server field
    let server_style = if app.connection_dialog.selected_field == 0 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let server_text = Paragraph::new(format!("Server:   {}", app.connection_dialog.server))
        .style(server_style);
    f.render_widget(server_text, form_chunks[0]);

    // Username field
    let user_style = if app.connection_dialog.selected_field == 1 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let user_text = Paragraph::new(format!("Username: {}", app.connection_dialog.username))
        .style(user_style);
    f.render_widget(user_text, form_chunks[1]);

    // Password field
    let pass_style = if app.connection_dialog.selected_field == 2 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let pass_text = Paragraph::new(format!("Password: {}", app.connection_dialog.password_mask()))
        .style(pass_style);
    f.render_widget(pass_text, form_chunks[2]);

    // Instructions
    let instructions = Paragraph::new("Tab: next field | Enter: connect | Esc: cancel")
        .style(Style::default().fg(Color::Gray))
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(instructions, form_chunks[3]);
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