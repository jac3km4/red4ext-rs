use thiserror::Error;

#[derive(Debug, Error)]
pub enum ResourcePathError {
    #[error("resource path should not be empty")]
    Empty,
    #[error("resource path should be inferior to {max} characters")]
    TooLong { max: usize },
    #[error("resource path should be an absolute path in archive e.g. 'base\\mod\\character.ent'")]
    Relative { path: String },
}
