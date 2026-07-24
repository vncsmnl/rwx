use clap::Parser;
use std::io;
use std::path::PathBuf;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

mod app;
mod permissions;
mod ui;

use app::{App, AppMode, Focusable};
use permissions::FilePermissions;

#[derive(Parser, Debug)]
#[command(
    name = "rwx",
    version,
    about = "Interactive file permissions TUI manager"
)]
struct Args {
    #[arg(help = "Path to the file or directory to inspect/edit")]
    path: Option<PathBuf>,

    #[arg(
        short = 'R',
        long = "recursive",
        help = "Apply changes recursively (has effect when target is a directory)"
    )]
    recursive: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Set panic hook to ensure terminal is restored if we crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    // Create app state
    let mut app = App::new().map_err(|e| format!("Failed to init app: {}", e))?;

    // Handle CLI arguments
    if let Some(target_path) = args.path {
        if target_path.symlink_metadata().is_ok() {
            if let Err(e) = app.open_target(target_path) {
                app.message = Some((e, true));
                app.show_popup = true;
            } else {
                app.recursive = args.recursive;
            }
        } else {
            app.message = Some((format!("Path does not exist: {:?}", target_path), true));
            app.show_popup = true;
        }
    }

    // Run TUI loop
    let res = run_loop(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {}", err);
    }

    Ok(())
}

fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal
            .draw(|f| ui::render(f, app))
            .map_err(|e| io::Error::other(e.to_string()))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                continue;
            }

            // 1. Popup mode
            if app.show_popup {
                match key.code {
                    KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => {
                        app.show_popup = false;
                        app.message = None;
                    }
                    _ => {}
                }
                continue;
            }

            // 2. Determine if we are actively typing in input field
            let is_input_focused = matches!(
                app.focus,
                Focusable::OctalInput | Focusable::OwnerInput | Focusable::GroupInput
            );

            // Handle keys
            match app.mode {
                AppMode::Browser => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(());
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.selected_item_idx > 0 {
                            app.selected_item_idx -= 1;
                            app.list_state.select(Some(app.selected_item_idx));
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if app.selected_item_idx + 1 < app.items.len() {
                            app.selected_item_idx += 1;
                            app.list_state.select(Some(app.selected_item_idx));
                        }
                    }
                    KeyCode::Enter => {
                        if !app.items.is_empty() {
                            let item = &app.items[app.selected_item_idx];
                            let path = item.path.clone();
                            if item.is_dir {
                                app.current_dir = path;
                                app.selected_item_idx = 0;
                                if let Err(e) = app.load_directory() {
                                    app.message = Some((e, true));
                                    app.show_popup = true;
                                }
                            } else if let Err(e) = app.open_target(path) {
                                app.message = Some((e, true));
                                app.show_popup = true;
                            }
                        }
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        let dir = app.current_dir.clone();
                        if let Err(e) = app.open_target(dir) {
                            app.message = Some((e, true));
                            app.show_popup = true;
                        }
                    }
                    _ => {}
                },
                AppMode::Editor => {
                    if is_input_focused
                        && !matches!(
                            key.code,
                            KeyCode::Esc
                                | KeyCode::Enter
                                | KeyCode::Up
                                | KeyCode::Down
                                | KeyCode::Tab
                                | KeyCode::BackTab
                        )
                    {
                        match key.code {
                            KeyCode::Char(c) => match app.focus {
                                Focusable::OctalInput => {
                                    if c.is_digit(8) && app.octal_input.len() < 4 {
                                        app.octal_input.push(c);
                                        if let Some(new_perms) =
                                            FilePermissions::from_octal_str(&app.octal_input)
                                        {
                                            app.permissions = new_perms;
                                        }
                                    }
                                }
                                Focusable::OwnerInput
                                    if c.is_alphanumeric() || c == '-' || c == '_' =>
                                {
                                    app.owner_input.push(c);
                                }
                                Focusable::GroupInput
                                    if c.is_alphanumeric() || c == '-' || c == '_' =>
                                {
                                    app.group_input.push(c);
                                }
                                _ => {}
                            },
                            KeyCode::Backspace => match app.focus {
                                Focusable::OctalInput => {
                                    app.octal_input.pop();
                                    if let Some(new_perms) =
                                        FilePermissions::from_octal_str(&app.octal_input)
                                    {
                                        app.permissions = new_perms;
                                    } else if app.octal_input.is_empty() {
                                        app.permissions = FilePermissions::from_mode(0);
                                    }
                                }
                                Focusable::OwnerInput => {
                                    app.owner_input.pop();
                                }
                                Focusable::GroupInput => {
                                    app.group_input.pop();
                                }
                                _ => {}
                            },
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') => {
                                return Ok(());
                            }
                            KeyCode::Esc => {
                                if is_input_focused {
                                    app.focus = Focusable::OwnerRead;
                                } else {
                                    return Ok(());
                                }
                            }
                            KeyCode::Char('b') | KeyCode::Char('B') | KeyCode::Backspace => {
                                if let Err(e) = app.load_directory() {
                                    app.message = Some((e, true));
                                    app.show_popup = true;
                                }
                                app.mode = AppMode::Browser;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                app.move_focus(0, -1);
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                app.move_focus(0, 1);
                            }
                            KeyCode::Left | KeyCode::Char('h') => {
                                app.move_focus(-1, 0);
                            }
                            KeyCode::Right | KeyCode::Char('l') => {
                                app.move_focus(1, 0);
                            }
                            KeyCode::Tab => {
                                app.move_focus(0, 1);
                            }
                            KeyCode::BackTab => {
                                app.move_focus(0, -1);
                            }
                            KeyCode::Char(' ') | KeyCode::Enter => {
                                match app.focus {
                                    Focusable::ApplyButton => {
                                        apply_action(app);
                                    }
                                    Focusable::QuitButton => {
                                        return Ok(());
                                    }
                                    Focusable::OctalInput
                                    | Focusable::OwnerInput
                                    | Focusable::GroupInput => {
                                        // Simple enter to start editing (not strictly needed since clicking it is enough)
                                    }
                                    _ => {
                                        app.toggle_current_checkbox();
                                    }
                                }
                            }
                            KeyCode::Char('a') | KeyCode::Char('A') => {
                                apply_action(app);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

fn apply_action(app: &mut App) {
    match app.apply_changes() {
        Ok(_) => {
            app.message = Some((
                "Permissions and ownership updated successfully!".to_string(),
                false,
            ));
            app.show_popup = true;
        }
        Err(e) => {
            app.message = Some((format!("Failed to apply: {}", e), true));
            app.show_popup = true;
        }
    }
}
