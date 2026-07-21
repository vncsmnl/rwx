use nix::unistd::{Gid, Group, Uid, User};
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::fs::chown;
use std::path::{Path, PathBuf};

use crate::permissions::FilePermissions;
use ratatui::widgets::ListState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Browser,
    Editor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focusable {
    OwnerRead,
    OwnerWrite,
    OwnerExecute,
    GroupRead,
    GroupWrite,
    GroupExecute,
    OthersRead,
    OthersWrite,
    OthersExecute,
    SetUid,
    SetGid,
    Sticky,
    OctalInput,
    OwnerInput,
    GroupInput,
    Recursive,
    ApplyButton,
    QuitButton,
}

pub struct BrowserItem {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
}

pub struct App {
    pub mode: AppMode,
    // File Browser state
    pub current_dir: PathBuf,
    pub items: Vec<BrowserItem>,
    pub selected_item_idx: usize,
    pub list_state: ListState,

    // Editor state
    pub target_path: PathBuf,
    pub is_dir: bool,
    pub file_size: u64,
    pub permissions: FilePermissions,

    // Input fields
    pub octal_input: String,
    pub owner_input: String,
    pub group_input: String,
    pub recursive: bool,

    // Original values (to detect changes)
    pub orig_permissions: FilePermissions,
    pub orig_owner: String,
    pub orig_group: String,

    // UI state
    pub focus: Focusable,
    pub message: Option<(String, bool)>, // (message text, is_error)
    pub show_popup: bool,
}

fn get_owner_name(uid: u32) -> String {
    User::from_uid(Uid::from_raw(uid))
        .ok()
        .flatten()
        .map(|u| u.name)
        .unwrap_or_else(|| uid.to_string())
}

fn get_group_name(gid: u32) -> String {
    Group::from_gid(Gid::from_raw(gid))
        .ok()
        .flatten()
        .map(|g| g.name)
        .unwrap_or_else(|| gid.to_string())
}

impl App {
    pub fn new() -> Result<Self, String> {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut app = Self {
            mode: AppMode::Browser,
            current_dir,
            items: Vec::new(),
            selected_item_idx: 0,
            list_state: ListState::default(),

            target_path: PathBuf::new(),
            is_dir: false,
            file_size: 0,
            permissions: FilePermissions::from_mode(0),

            octal_input: String::new(),
            owner_input: String::new(),
            group_input: String::new(),
            recursive: false,

            orig_permissions: FilePermissions::from_mode(0),
            orig_owner: String::new(),
            orig_group: String::new(),

            focus: Focusable::OwnerRead,
            message: None,
            show_popup: false,
        };
        app.load_directory()?;
        Ok(app)
    }

    pub fn load_directory(&mut self) -> Result<(), String> {
        self.items.clear();

        // Add parent option if current dir has a parent
        if let Some(parent) = self.current_dir.parent() {
            self.items.push(BrowserItem {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                is_dir: true,
                size: 0,
            });
        }

        let entries = std::fs::read_dir(&self.current_dir)
            .map_err(|e| format!("Failed to read directory: {}", e))?;

        let mut temp_items = Vec::new();
        for entry in entries.flatten() {
            let metadata = entry.metadata().ok();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
            let name = entry.file_name().to_string_lossy().to_string();

            temp_items.push(BrowserItem {
                name,
                path: entry.path(),
                is_dir,
                size,
            });
        }

        // Sort: directories first (alphabetical), then files (alphabetical)
        temp_items.sort_by(|a, b| {
            if a.is_dir && !b.is_dir {
                std::cmp::Ordering::Less
            } else if !a.is_dir && b.is_dir {
                std::cmp::Ordering::Greater
            } else {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }
        });

        self.items.extend(temp_items);

        // Adjust selected index
        if self.items.is_empty() {
            self.selected_item_idx = 0;
        } else if self.selected_item_idx >= self.items.len() {
            self.selected_item_idx = self.items.len() - 1;
        }
        self.list_state.select(Some(self.selected_item_idx));

        Ok(())
    }

    pub fn open_target(&mut self, path: PathBuf) -> Result<(), String> {
        let metadata = std::fs::metadata(&path)
            .map_err(|e| format!("Failed to read metadata for {:?}: {}", path, e))?;

        self.target_path = path;
        self.is_dir = metadata.is_dir();
        self.file_size = metadata.len();

        let mode = metadata.permissions().mode();
        self.permissions = FilePermissions::from_mode(mode);
        self.orig_permissions = self.permissions;

        self.octal_input = self.permissions.to_octal();

        let uid = metadata.uid();
        let gid = metadata.gid();

        self.owner_input = get_owner_name(uid);
        self.orig_owner = self.owner_input.clone();

        self.group_input = get_group_name(gid);
        self.orig_group = self.group_input.clone();

        self.focus = Focusable::OwnerRead;
        self.mode = AppMode::Editor;
        self.message = None;
        self.show_popup = false;

        Ok(())
    }

    pub fn toggle_current_checkbox(&mut self) {
        match self.focus {
            Focusable::OwnerRead => self.permissions.owner.read = !self.permissions.owner.read,
            Focusable::OwnerWrite => self.permissions.owner.write = !self.permissions.owner.write,
            Focusable::OwnerExecute => {
                self.permissions.owner.execute = !self.permissions.owner.execute
            }
            Focusable::GroupRead => self.permissions.group.read = !self.permissions.group.read,
            Focusable::GroupWrite => self.permissions.group.write = !self.permissions.group.write,
            Focusable::GroupExecute => {
                self.permissions.group.execute = !self.permissions.group.execute
            }
            Focusable::OthersRead => self.permissions.others.read = !self.permissions.others.read,
            Focusable::OthersWrite => {
                self.permissions.others.write = !self.permissions.others.write
            }
            Focusable::OthersExecute => {
                self.permissions.others.execute = !self.permissions.others.execute
            }
            Focusable::SetUid => self.permissions.setuid = !self.permissions.setuid,
            Focusable::SetGid => self.permissions.setgid = !self.permissions.setgid,
            Focusable::Sticky => self.permissions.sticky = !self.permissions.sticky,
            Focusable::Recursive => self.recursive = !self.recursive,
            _ => {}
        }
        self.octal_input = self.permissions.to_octal();
    }

    pub fn move_focus(&mut self, dx: i32, dy: i32) {
        let (mut x, mut y) = match self.focus {
            Focusable::OwnerRead => (0, 0),
            Focusable::OwnerWrite => (1, 0),
            Focusable::OwnerExecute => (2, 0),
            Focusable::GroupRead => (0, 1),
            Focusable::GroupWrite => (1, 1),
            Focusable::GroupExecute => (2, 1),
            Focusable::OthersRead => (0, 2),
            Focusable::OthersWrite => (1, 2),
            Focusable::OthersExecute => (2, 2),
            Focusable::SetUid => (0, 3),
            Focusable::SetGid => (1, 3),
            Focusable::Sticky => (2, 3),
            Focusable::OctalInput => (0, 4),
            Focusable::OwnerInput => (0, 5),
            Focusable::GroupInput => (1, 5),
            Focusable::Recursive => (0, 6),
            Focusable::ApplyButton => (0, 7),
            Focusable::QuitButton => (1, 7),
        };

        x = (x + dx).max(0);
        y = (y + dy).max(0);

        if y > 7 {
            y = 7;
        }

        match y {
            0..=3 => x = x.min(2),
            4 | 6 => x = 0,
            5 | 7 => x = x.min(1),
            _ => {}
        }

        self.focus = match (x, y) {
            (0, 0) => Focusable::OwnerRead,
            (1, 0) => Focusable::OwnerWrite,
            (2, 0) => Focusable::OwnerExecute,
            (0, 1) => Focusable::GroupRead,
            (1, 1) => Focusable::GroupWrite,
            (2, 1) => Focusable::GroupExecute,
            (0, 2) => Focusable::OthersRead,
            (1, 2) => Focusable::OthersWrite,
            (2, 2) => Focusable::OthersExecute,
            (0, 3) => Focusable::SetUid,
            (1, 3) => Focusable::SetGid,
            (2, 3) => Focusable::Sticky,
            (0, 4) => Focusable::OctalInput,
            (0, 5) => Focusable::OwnerInput,
            (1, 5) => Focusable::GroupInput,
            (_, 6) => Focusable::Recursive,
            (0, 7) => Focusable::ApplyButton,
            (_, 7) => Focusable::QuitButton,
            _ => Focusable::OwnerRead,
        };
    }

    pub fn validate_owner(&self) -> bool {
        if self.owner_input.is_empty() {
            return true;
        }
        if self.owner_input.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
        User::from_name(&self.owner_input)
            .map(|u| u.is_some())
            .unwrap_or(false)
    }

    pub fn validate_group(&self) -> bool {
        if self.group_input.is_empty() {
            return true;
        }
        if self.group_input.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }
        Group::from_name(&self.group_input)
            .map(|g| g.is_some())
            .unwrap_or(false)
    }

    pub fn apply_changes(&mut self) -> Result<(), String> {
        let mode = self.permissions.to_mode();

        let uid = if self.owner_input == self.orig_owner || self.owner_input.is_empty() {
            None
        } else if let Ok(parsed_uid) = self.owner_input.parse::<u32>() {
            Some(Uid::from_raw(parsed_uid))
        } else if let Ok(Some(u)) = User::from_name(&self.owner_input) {
            Some(u.uid)
        } else {
            return Err(format!("Invalid owner name: '{}'", self.owner_input));
        };

        let gid = if self.group_input == self.orig_group || self.group_input.is_empty() {
            None
        } else if let Ok(parsed_gid) = self.group_input.parse::<u32>() {
            Some(Gid::from_raw(parsed_gid))
        } else if let Ok(Some(g)) = Group::from_name(&self.group_input) {
            Some(g.gid)
        } else {
            return Err(format!("Invalid group name: '{}'", self.group_input));
        };

        let apply_to = |p: &Path| -> Result<(), std::io::Error> {
            let perms = std::fs::Permissions::from_mode(mode);
            std::fs::set_permissions(p, perms)?;

            if uid.is_some() || gid.is_some() {
                let u_raw = uid.map(|u| u.as_raw());
                let g_raw = gid.map(|g| g.as_raw());
                chown(p, u_raw, g_raw)?;
            }
            Ok(())
        };

        if self.recursive && self.is_dir {
            for entry in walkdir::WalkDir::new(&self.target_path) {
                let entry =
                    entry.map_err(|e| format!("Error reading directory structure: {}", e))?;
                apply_to(entry.path())
                    .map_err(|e| format!("Failed to apply to {:?}: {}", entry.path(), e))?;
            }
        } else {
            apply_to(&self.target_path).map_err(|e| format!("Failed to apply changes: {}", e))?;
        }

        self.orig_permissions = self.permissions;
        if uid.is_some() {
            self.orig_owner = self.owner_input.clone();
        }
        if gid.is_some() {
            self.orig_group = self.group_input.clone();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> App {
        App {
            mode: AppMode::Editor,
            current_dir: PathBuf::from("."),
            items: Vec::new(),
            selected_item_idx: 0,
            list_state: ListState::default(),
            target_path: PathBuf::from("test.txt"),
            is_dir: false,
            file_size: 100,
            permissions: FilePermissions::from_mode(0o644),
            octal_input: "0644".to_string(),
            owner_input: "root".to_string(),
            group_input: "root".to_string(),
            recursive: false,
            orig_permissions: FilePermissions::from_mode(0o644),
            orig_owner: "root".to_string(),
            orig_group: "root".to_string(),
            focus: Focusable::OwnerRead,
            message: None,
            show_popup: false,
        }
    }

    #[test]
    fn test_toggle_checkboxes() {
        let mut app = create_test_app();

        app.focus = Focusable::OwnerRead;
        app.toggle_current_checkbox();
        assert!(!app.permissions.owner.read);
        assert_eq!(app.octal_input, "0244");

        app.focus = Focusable::OwnerExecute;
        app.toggle_current_checkbox();
        assert!(app.permissions.owner.execute);
        assert_eq!(app.octal_input, "0344");

        app.focus = Focusable::SetUid;
        app.toggle_current_checkbox();
        assert!(app.permissions.setuid);
        assert_eq!(app.octal_input, "4344");

        app.focus = Focusable::Recursive;
        app.toggle_current_checkbox();
        assert!(app.recursive);
    }

    #[test]
    fn test_move_focus_navigation() {
        let mut app = create_test_app();
        app.focus = Focusable::OwnerRead;

        app.move_focus(1, 0);
        assert_eq!(app.focus, Focusable::OwnerWrite);

        app.move_focus(1, 0);
        assert_eq!(app.focus, Focusable::OwnerExecute);

        app.move_focus(0, 1);
        assert_eq!(app.focus, Focusable::GroupExecute);

        app.move_focus(0, 3); // row 4 => OctalInput
        assert_eq!(app.focus, Focusable::OctalInput);

        app.move_focus(0, 1); // row 5 => OwnerInput
        assert_eq!(app.focus, Focusable::OwnerInput);

        app.move_focus(1, 0); // row 5, col 1 => GroupInput
        assert_eq!(app.focus, Focusable::GroupInput);

        app.move_focus(0, 2); // row 7 => Apply / Quit
        assert_eq!(app.focus, Focusable::QuitButton);
    }

    #[test]
    fn test_validation() {
        let mut app = create_test_app();

        app.owner_input = "".to_string();
        assert!(app.validate_owner());

        app.owner_input = "1000".to_string();
        assert!(app.validate_owner());

        app.group_input = "".to_string();
        assert!(app.validate_group());

        app.group_input = "1000".to_string();
        assert!(app.validate_group());
    }
}
