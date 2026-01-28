use std::io;
use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Invalid date format: {0}")]
    InvalidDate(String),
    
    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(#[from] regex::Error),
    
    #[error("File access denied: {0}")]
    AccessDenied(PathBuf),
    
    #[error("Cannot delete file: {0}")]
    DeleteError(String),
    
    #[error("Invalid sort combination: {0}")]
    InvalidSort(String),
    
    #[error("Walkdir error: {0}")]
    Walkdir(#[from] walkdir::Error),
    
    #[error("Dialoguer error: {0}")]
    Dialoguer(#[from] dialoguer::Error),
    
    #[error("Confy error: {0}")]
    Confy(#[from] confy::ConfyError),
}

pub type Result<T> = std::result::Result<T, AppError>;