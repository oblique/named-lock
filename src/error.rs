use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to create named lock")]
    CreateFailed,

    #[error("Failed to lock named lock")]
    LockFailed,

    #[error("Failed to unlock named lock")]
    UnlockFailed,

    #[error("Named lock would block")]
    WouldBlock,
}
