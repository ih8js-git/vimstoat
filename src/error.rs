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

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Keyring Error: {0:?}")]
    KeyringError(keyring::Error),

    #[error("Invalid Token: {0}")]
    InvalidToken(String),

    #[error("Could not connect to the server. Please check your internet connection.")]
    ServerConnectionError,

    #[error("Request Error: {0}")]
    RequestError(String),
}

impl From<keyring::Error> for AuthError {
    fn from(value: keyring::Error) -> Self {
        AuthError::KeyringError(value)
    }
}
