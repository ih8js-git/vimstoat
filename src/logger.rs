use std::{fs, path::PathBuf};

pub use crate::LOG_FILE;
use crate::Result;

pub fn create_log_file() -> Result<fs::File> {
    let mut path = if let Some(mut p) = dirs::cache_dir() {
        p.push(env!("CARGO_PKG_NAME"));
        p
    } else {
        PathBuf::new()
    };

    fs::create_dir_all(&path)?;

    path.push(LOG_FILE);

    Ok(fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("Failed to open log file"))
}
