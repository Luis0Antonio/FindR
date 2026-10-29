use serde::Serialize;
use std::io;

#[derive(Debug, thiserror::Error, Serialize, Clone, PartialEq, Eq)]
#[serde(tag = "code", content = "message", rename_all = "snake_case")]
pub enum FsError {
    #[error("{0}")]
    PermissionDenied(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Exists(String),
    #[error("{0}")]
    InvalidName(String),
    #[error("{0}")]
    InvalidTarget(String),
    #[error("{0}")]
    Io(String),
}

pub type FsResult<T> = Result<T, FsError>;

impl FsError {
    pub fn from_io(err: io::Error, context: &str) -> Self {
        let msg = format!("{context}: {err}");
        match err.kind() {
            io::ErrorKind::PermissionDenied => FsError::PermissionDenied(msg),
            io::ErrorKind::NotFound => FsError::NotFound(msg),
            io::ErrorKind::AlreadyExists => FsError::Exists(msg),
            _ if err.raw_os_error() == Some(libc::EPERM) => FsError::PermissionDenied(msg),
            _ => FsError::Io(msg),
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            FsError::PermissionDenied(_) => "permission_denied",
            FsError::NotFound(_) => "not_found",
            FsError::Exists(_) => "exists",
            FsError::InvalidName(_) => "invalid_name",
            FsError::InvalidTarget(_) => "invalid_target",
            FsError::Io(_) => "io",
        }
    }
}
