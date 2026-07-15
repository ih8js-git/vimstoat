use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Database Error: {0:?}")]
    DbError(pickledb::error::Error),

    #[error("Directory not found: {0}")]
    DirNotFound(String),
}

#[derive(Error, Debug)]
pub enum IdError {
    #[error("Invalid size: expected 26, got {0}")]
    InvalidSize(usize),
}

impl From<pickledb::error::Error> for CacheError {
    fn from(value: pickledb::error::Error) -> Self {
        CacheError::DbError(value)
    }
}
