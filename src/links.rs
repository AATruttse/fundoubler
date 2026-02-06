//! Link creation: symlinks, hardlinks, Windows shortcuts.

use std::path::{Path, PathBuf};
use crate::error::{AppError, Result};

/// Determine link target path based on config
pub fn get_link_path(deleted: &Path, kept: &Path, no_keep_link_names: bool, is_shortcut: bool) -> PathBuf {
    if no_keep_link_names && !is_shortcut {
        // Use kept file's name but place it where deleted file was
        if let Some(parent) = deleted.parent() {
            if let Some(name) = kept.file_name() {
                parent.join(name)
            } else {
                deleted.to_path_buf()
            }
        } else {
            // No parent, use kept's name in current dir
            kept.file_name()
                .map(|n| PathBuf::from(n))
                .unwrap_or_else(|| deleted.to_path_buf())
        }
    } else if is_shortcut {
        // Shortcuts always get .lnk extension
        if no_keep_link_names {
            // Use kept file's name + .lnk, placed where deleted file was
            if let Some(parent) = deleted.parent() {
                if let Some(name) = kept.file_name() {
                    parent.join(name).with_extension("lnk")
                } else {
                    deleted.with_extension("lnk")
                }
            } else {
                kept.file_name()
                    .map(|n| PathBuf::from(n).with_extension("lnk"))
                    .unwrap_or_else(|| deleted.with_extension("lnk"))
            }
        } else {
            // Use deleted file's name + .lnk
            deleted.with_extension("lnk")
        }
    } else {
        // Use deleted file's name for symlinks/hardlinks (same location)
        deleted.to_path_buf()
    }
}

/// Create a symlink pointing to target at link_path
pub fn create_symlink(target: &Path, link_path: &Path) -> Result<()> {
    // Remove existing file/link if it exists
    if link_path.exists() {
        std::fs::remove_file(link_path)?;
    }

    // Create parent directories if needed
    if let Some(parent) = link_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        symlink(target, link_path)?;
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::symlink_file;
        // On Windows, symlink_file requires admin or developer mode
        // Fall back to junction or shortcut if symlink fails
        if symlink_file(target, link_path).is_err() {
            // Try creating a junction (directory symlink) or fall back to shortcut
            return Err(AppError::Config(format!(
                "Failed to create symlink at {}: requires admin privileges or developer mode",
                link_path.display()
            )));
        }
    }

    Ok(())
}

/// Create a hardlink pointing to target at link_path
pub fn create_hardlink(target: &Path, link_path: &Path) -> Result<()> {
    // Remove existing file/link if it exists
    if link_path.exists() {
        std::fs::remove_file(link_path)?;
    }

    // Create parent directories if needed
    if let Some(parent) = link_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::hard_link(target, link_path)?;
    Ok(())
}

/// Create a Windows shortcut (.lnk file) pointing to target at link_path
#[cfg(windows)]
pub fn create_shortcut(target: &Path, link_path: &Path) -> Result<()> {
    use std::fs;
    use mslnk::ShellLink;

    // Remove existing file if it exists
    if link_path.exists() {
        fs::remove_file(link_path)?;
    }

    // Create parent directories if needed
    if let Some(parent) = link_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Get absolute path for target
    let target_abs = fs::canonicalize(target)
        .map_err(|e| AppError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Cannot create shortcut: target {} not found: {}", target.display(), e)
        )))?;

    // Create shortcut - convert PathBuf to String
    let target_str = target_abs.to_string_lossy().to_string();
    let shortcut = ShellLink::new(&target_str)
        .map_err(|e| AppError::Config(format!("Failed to create shortcut at {}: {}", link_path.display(), e)))?;
    shortcut.create_lnk(link_path)
        .map_err(|e| AppError::Config(format!("Failed to save shortcut at {}: {}", link_path.display(), e)))?;

    Ok(())
}

#[cfg(not(windows))]
pub fn create_shortcut(_target: &Path, link_path: &Path) -> Result<()> {
    Err(AppError::Config(format!(
        "Windows shortcuts are only supported on Windows: {}",
        link_path.display()
    )))
}

/// Get link type description for display
pub fn get_link_type_description(create_symlinks: bool, create_hardlinks: bool, create_shortcuts: bool) -> Option<&'static str> {
    if create_symlinks {
        Some("symlink")
    } else if create_hardlinks {
        Some("hardlink")
    } else if create_shortcuts {
        Some("shortcut")
    } else {
        None
    }
}
