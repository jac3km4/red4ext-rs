use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResourcePathError {
    #[error("resource path should not be empty")]
    Empty,
    #[error("resource path should be inferior to {max} characters")]
    TooLong { max: usize },
}
