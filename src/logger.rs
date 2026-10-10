use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::Result;

pub const LOG_FILE: &str = "logs";

fn log_path(cache_dir: Option<PathBuf>) -> PathBuf {
    let mut path = if let Some(mut p) = cache_dir {
        p.push(env!("CARGO_PKG_NAME"));
        p
    } else {
        PathBuf::new()
    };

    path.push(LOG_FILE);
    path
}

fn open_log_file(path: &Path) -> Result<fs::File> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    Ok(fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("Failed to open log file"))
}

fn create_log_file() -> Result<fs::File> {
    open_log_file(&log_path(dirs::cache_dir()))
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

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn log_path_uses_package_name_and_log_file() {
        let base = PathBuf::from("/some/cache");
        let path = log_path(Some(base.clone()));

        assert_eq!(path, base.join(env!("CARGO_PKG_NAME")).join(LOG_FILE));
    }

    #[test]
    fn log_path_without_cache_dir_is_relative() {
        assert_eq!(log_path(None), PathBuf::from(LOG_FILE));
    }

    #[test]
    fn open_log_file_creates_missing_directories() {
        let tmp = TempDir::new().unwrap();
        let path = log_path(Some(tmp.path().join("nested").join("cache")));
        assert!(!path.parent().unwrap().exists());

        open_log_file(&path).unwrap();

        assert!(path.is_file());
    }

    #[test]
    fn open_log_file_appends_across_opens() {
        let tmp = TempDir::new().unwrap();
        let path = log_path(Some(tmp.path().to_path_buf()));

        let mut f = open_log_file(&path).unwrap();
        f.write_all(b"a").unwrap();
        drop(f);

        let mut f = open_log_file(&path).unwrap();
        f.write_all(b"b").unwrap();
        drop(f);

        assert_eq!(fs::read_to_string(&path).unwrap(), "ab");
    }

    #[test]
    fn open_log_file_preserves_existing_content() {
        let tmp = TempDir::new().unwrap();
        let path = log_path(Some(tmp.path().to_path_buf()));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "existing\n").unwrap();

        let mut f = open_log_file(&path).unwrap();
        f.write_all(b"new\n").unwrap();
        drop(f);

        assert_eq!(fs::read_to_string(&path).unwrap(), "existing\nnew\n");
    }
}
