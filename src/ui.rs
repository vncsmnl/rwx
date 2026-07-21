use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table},
    Frame,
};

use crate::app::{App, AppMode, Focusable};

pub fn render(f: &mut Frame, app: &mut App) {
    match app.mode {
        AppMode::Browser => render_browser(f, app),
        AppMode::Editor => render_editor(f, app),
    }

    if app.show_popup {
        render_popup(f, app);
    }
}

fn draw_checkbox(checked: bool, is_focused: bool) -> Span<'static> {
    let check_char = if checked { "✓" } else { " " };
    if is_focused {
        Span::styled(
            format!(" [{}] ", check_char),
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(
            format!(" [{}] ", check_char),
            if checked {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        )
    }
}

fn render_browser(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // File List
            Constraint::Length(3), // Footer / Help Bar
        ])
        .split(f.area());

    // 1. Header
    let header_text = vec![
        Line::from(vec![
            Span::styled(" rwx ", Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::raw(" | Directory Selector: "),
            Span::styled(app.current_dir.to_string_lossy().to_string(), Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ])
    ];
    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Blue)));
    f.render_widget(header, chunks[0]);

    // 2. File List
    let items: Vec<ListItem> = app
        .items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_selected = idx == app.selected_item_idx;
            
            let prefix = if is_selected {
                Span::styled("  ▶ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            } else {
                Span::raw("    ")
            };

            let name_span = if item.is_dir {
                Span::styled(
                    format!("{}/", item.name),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(&item.name, Style::default().fg(Color::White))
            };

            let size_str = if item.is_dir {
                String::new()
            } else {
                format_size(item.size)
            };
            let size_span = Span::styled(
                format!(" {:>10}", size_str),
                Style::default().fg(Color::DarkGray),
            );

            let mut style = Style::default();
            if is_selected {
                style = style.bg(Color::Rgb(30, 30, 46));
            }

            ListItem::new(Line::from(vec![prefix, name_span, size_span])).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Files and Directories ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_stateful_widget(list, chunks[1], &mut app.list_state);

    // 3. Footer Help Bar
    let footer_text = Line::from(vec![
        Span::styled(" [↑/↓, j/k] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("Navigate  "),
        Span::styled(" [Enter] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("Open/Select  "),
        Span::styled(" [S] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("Edit current folder  "),
        Span::styled(" [Q] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("Quit"),
    ]);
    let footer = Paragraph::new(footer_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(footer, chunks[2]);
}

fn render_editor(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header info
            Constraint::Min(14),   // Main dashboard (split horizontally)
            Constraint::Length(3), // Footer buttons & shortcuts
        ])
        .split(f.area());

    // 1. Header (File Info)
    let file_type = if app.is_dir { "Directory" } else { "File" };
    let size_str = format_size(app.file_size);
    let header_text = vec![
        Line::from(vec![
            Span::styled(" Target: ", Style::default().fg(Color::DarkGray)),
            Span::styled(app.target_path.file_name().unwrap_or_default().to_string_lossy().to_string(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" ({})", file_type), Style::default().fg(Color::Cyan)),
            Span::raw(" | Size: "),
            Span::styled(size_str, Style::default().fg(Color::White)),
            Span::raw(" | Full Path: "),
            Span::styled(app.target_path.to_string_lossy().to_string(), Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)),
        ])
    ];
    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Color::Blue)));
    f.render_widget(header, chunks[0]);

    // Split main section vertically into left (permissions grid) and right (metadata/inputs)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55), // Permissions Grid
            Constraint::Percentage(45), // Octal/Symbolic + Chown/Chgrp
        ])
        .split(chunks[1]);

    // LEFT PANEL: Permissions Grid
    let perm_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Grid Table
            Constraint::Length(3), // Special bits row
            Constraint::Length(3), // Recursive option
        ])
        .split(main_chunks[0]);

    // Permissions Grid Table
    let header_row = Row::new(vec![
        Cell::from(""),
        Cell::from(Line::from("Read (r)").alignment(Alignment::Center)),
        Cell::from(Line::from("Write (w)").alignment(Alignment::Center)),
        Cell::from(Line::from("Execute (x)").alignment(Alignment::Center)),
    ])
    .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let rows = vec![
        Row::new(vec![
            Cell::from(" Owner (u) ").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Cell::from(Line::from(draw_checkbox(app.permissions.owner.read, app.focus == Focusable::OwnerRead)).alignment(Alignment::Center)),
            Cell::from(Line::from(draw_checkbox(app.permissions.owner.write, app.focus == Focusable::OwnerWrite)).alignment(Alignment::Center)),
            Cell::from(Line::from(draw_checkbox(app.permissions.owner.execute, app.focus == Focusable::OwnerExecute)).alignment(Alignment::Center)),
        ]),
        Row::new(vec![
            Cell::from(" Group (g) ").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Cell::from(Line::from(draw_checkbox(app.permissions.group.read, app.focus == Focusable::GroupRead)).alignment(Alignment::Center)),
            Cell::from(Line::from(draw_checkbox(app.permissions.group.write, app.focus == Focusable::GroupWrite)).alignment(Alignment::Center)),
            Cell::from(Line::from(draw_checkbox(app.permissions.group.execute, app.focus == Focusable::GroupExecute)).alignment(Alignment::Center)),
        ]),
        Row::new(vec![
            Cell::from(" Others (o) ").style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Cell::from(Line::from(draw_checkbox(app.permissions.others.read, app.focus == Focusable::OthersRead)).alignment(Alignment::Center)),
            Cell::from(Line::from(draw_checkbox(app.permissions.others.write, app.focus == Focusable::OthersWrite)).alignment(Alignment::Center)),
            Cell::from(Line::from(draw_checkbox(app.permissions.others.execute, app.focus == Focusable::OthersExecute)).alignment(Alignment::Center)),
        ]),
    ];

    let grid_table = Table::new(rows, [
        Constraint::Length(14),
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Length(12),
    ])
    .header(header_row)
    .block(
        Block::default()
            .title(" Permissions Grid ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(grid_table, perm_layout[0]);

    // Special Bits Row
    let spec_uid = draw_checkbox(app.permissions.setuid, app.focus == Focusable::SetUid);
    let spec_gid = draw_checkbox(app.permissions.setgid, app.focus == Focusable::SetGid);
    let spec_sticky = draw_checkbox(app.permissions.sticky, app.focus == Focusable::Sticky);

    let special_line = Line::from(vec![
        Span::raw("  SetUID:"),
        spec_uid,
        Span::raw("    SetGID:"),
        spec_gid,
        Span::raw("    Sticky:"),
        spec_sticky,
    ]);

    let special_block = Paragraph::new(special_line)
        .block(
            Block::default()
                .title(" Special Bits ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(special_block, perm_layout[1]);

    // Recursive Toggle Checkbox
    let rec_checkbox = draw_checkbox(app.recursive, app.focus == Focusable::Recursive);
    let rec_line = Line::from(vec![
        rec_checkbox,
        Span::raw(" Apply recursively (chown/chmod) to all contents"),
    ]);
    let rec_block = Paragraph::new(rec_line)
        .block(
            Block::default()
                .title(" Options ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(rec_block, perm_layout[2]);


    // RIGHT PANEL: Representation & Ownership Inputs
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Octal Input
            Constraint::Length(5), // Symbolic output & changes preview
            Constraint::Length(6), // Owner & Group Inputs
        ])
        .split(main_chunks[1]);

    // Octal Input field
    let is_octal_focused = app.focus == Focusable::OctalInput;
    let octal_display = if is_octal_focused {
        format!("{}_", app.octal_input)
    } else {
        app.octal_input.clone()
    };
    
    // Check if the input is currently valid
    let octal_valid = u32::from_str_radix(&app.octal_input, 8).is_ok() && app.octal_input.len() >= 3 && app.octal_input.len() <= 4;
    let octal_border_style = if is_octal_focused {
        Style::default().fg(Color::Cyan)
    } else if !octal_valid && !app.octal_input.is_empty() {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let octal_para = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled(octal_display, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        if !octal_valid && !app.octal_input.is_empty() {
            Span::styled(" (invalid octal)", Style::default().fg(Color::Red).add_modifier(Modifier::ITALIC))
        } else {
            Span::raw("")
        }
    ]))
    .block(
        Block::default()
            .title(" Octal Value ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(octal_border_style),
    );
    f.render_widget(octal_para, right_chunks[0]);

    // Symbolic & Info Box
    let sym_repr = app.permissions.to_symbolic(app.is_dir);
    let mut info_lines = vec![
        Line::from(vec![
            Span::raw("  Symbolic: "),
            Span::styled(sym_repr, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::raw("  Current:  "),
            Span::styled(app.orig_permissions.to_octal(), Style::default().fg(Color::DarkGray)),
            Span::raw(" ("),
            Span::styled(app.orig_permissions.to_symbolic(app.is_dir), Style::default().fg(Color::DarkGray)),
            Span::raw(")"),
        ])
    ];

    // Detect if changes were made
    let permissions_changed = app.permissions != app.orig_permissions;
    let owner_changed = app.owner_input != app.orig_owner;
    let group_changed = app.group_input != app.orig_group;
    let changes_pending = permissions_changed || owner_changed || group_changed;

    if changes_pending {
        info_lines.push(Line::from(vec![
            Span::styled("  ⚠️  Pending Changes", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]));
    } else {
        info_lines.push(Line::from(vec![
            Span::styled("  ✓  Saved", Style::default().fg(Color::DarkGray)),
        ]));
    }

    let info_para = Paragraph::new(info_lines)
        .block(
            Block::default()
                .title(" Symbolic & Status ")
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(info_para, right_chunks[1]);

    // Owner and Group Inputs
    let owner_focused = app.focus == Focusable::OwnerInput;
    let group_focused = app.focus == Focusable::GroupInput;

    let owner_display = if owner_focused {
        format!("{}_", app.owner_input)
    } else {
        app.owner_input.clone()
    };
    let group_display = if group_focused {
        format!("{}_", app.group_input)
    } else {
        app.group_input.clone()
    };

    let owner_valid = app.validate_owner();
    let group_valid = app.validate_group();

    let owner_style = if !owner_valid {
        Style::default().fg(Color::Red)
    } else if owner_changed {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    let group_style = if !group_valid {
        Style::default().fg(Color::Red)
    } else if group_changed {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    let owner_border = if owner_focused {
        Style::default().fg(Color::Cyan)
    } else if !owner_valid {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let group_border = if group_focused {
        Style::default().fg(Color::Cyan)
    } else if !group_valid {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // Sub-layout for owner/group side-by-side or stacked. Let's stack them vertically inside the block
    let owner_group_block = Block::default()
        .title(" Ownership (chown / chgrp) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));
    
    // We can draw paragraphs for each inside the inner area
    let inner_rect = owner_group_block.inner(right_chunks[2]);
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Owner
            Constraint::Length(2), // Group
        ])
        .split(inner_rect);

    let owner_para = Paragraph::new(Line::from(vec![
        Span::raw(" Owner: "),
        Span::styled(owner_display, owner_style),
        if !owner_valid {
            Span::styled(" (unknown user)", Style::default().fg(Color::Red).add_modifier(Modifier::ITALIC))
        } else if owner_changed {
            Span::styled(" (changed)", Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC))
        } else {
            Span::raw("")
        }
    ]))
    .block(Block::default().borders(Borders::BOTTOM).border_style(owner_border));

    let group_para = Paragraph::new(Line::from(vec![
        Span::raw(" Group: "),
        Span::styled(group_display, group_style),
        if !group_valid {
            Span::styled(" (unknown group)", Style::default().fg(Color::Red).add_modifier(Modifier::ITALIC))
        } else if group_changed {
            Span::styled(" (changed)", Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC))
        } else {
            Span::raw("")
        }
    ]))
    .block(Block::default().border_style(group_border));

    f.render_widget(owner_group_block, right_chunks[2]);
    f.render_widget(owner_para, inner_chunks[0]);
    f.render_widget(group_para, inner_chunks[1]);


    // 3. Footer (Buttons + Shortcut Bar)
    let footer_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(16), // Apply Button
            Constraint::Length(16), // Quit Button
            Constraint::Min(20),    // Help Bar
        ])
        .split(chunks[2]);

    // Apply Button
    let apply_focused = app.focus == Focusable::ApplyButton;
    let apply_btn = Paragraph::new(" [A] Apply ")
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(if apply_focused {
                    Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD)
                } else if changes_pending {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
        );
    f.render_widget(apply_btn, footer_chunks[0]);

    // Quit Button
    let quit_focused = app.focus == Focusable::QuitButton;
    let quit_btn = Paragraph::new(" [Q] Quit ")
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(if quit_focused {
                    Style::default().bg(Color::Red).fg(Color::Black).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
        );
    f.render_widget(quit_btn, footer_chunks[1]);

    // Keyboard navigation help
    let shortcut_text = Line::from(vec![
        Span::styled(" [Arrows/Tab] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("Navigate  "),
        Span::styled(" [Space] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("Toggle checkbox  "),
        Span::styled(" [B] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("Back to browser  "),
        Span::styled(" [755, 644] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("Edit Octal directly"),
    ]);
    let shortcut_para = Paragraph::new(shortcut_text)
        .alignment(Alignment::Right)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(shortcut_para, footer_chunks[2]);
}

fn render_popup(f: &mut Frame, app: &App) {
    let size = f.area();
    let popup_block = Block::default()
        .title(" System Message ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow));

    // Get message details
    let (msg, is_error) = match &app.message {
        Some((m, err)) => (m.as_str(), *err),
        None => ("", false),
    };

    let title_style = if is_error {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    };

    let status_str = if is_error { "ERROR" } else { "INFO" };

    let content = Text::from(vec![
        Line::from(vec![Span::styled(status_str, title_style)]),
        Line::from(""),
        Line::from(msg),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Press ", Style::default().fg(Color::DarkGray)),
            Span::styled(" [Esc] / [Enter] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::styled(" to close ", Style::default().fg(Color::DarkGray)),
        ]),
    ]);

    let paragraph = Paragraph::new(content)
        .alignment(Alignment::Center)
        .block(popup_block);

    // Position popup in center (approx 50% width, 30% height)
    let area = centered_rect(60, 25, size);
    f.render_widget(Clear, area); // Clear background under popup
    f.render_widget(paragraph, area);
}

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

fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    let kilobytes = bytes as f64 / 1024.0;
    let megabytes = kilobytes / 1024.0;
    let gigabytes = megabytes / 1024.0;

    if gigabytes >= 1.0 {
        format!("{:.1} GB", gigabytes)
    } else if megabytes >= 1.0 {
        format!("{:.1} MB", megabytes)
    } else if kilobytes >= 1.0 {
        format!("{:.1} KB", kilobytes)
    } else {
        format!("{} B", bytes)
    }
}
