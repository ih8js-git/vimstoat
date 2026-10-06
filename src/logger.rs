use std::{fs, path::PathBuf};

use crate::Result;

pub const LOG_FILE: &str = "logs";

fn create_log_file() -> Result<fs::File> {
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

pub fn init() -> Result<()> {
    let log_file = create_log_file()?;

    log::info!("Starting vimstoat.");

    env_logger::builder()
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .filter_level(log::LevelFilter::Debug)
        .init();

    Ok(())
}
