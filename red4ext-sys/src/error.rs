use thiserror::Error;

use crate::interop::ResourcePath;

#[derive(Debug, Error)]
pub enum ResourcePathError {
    #[error("resource path should not be empty")]
    Empty,
    #[error(
        "resource path should be less than {} characters",
        ResourcePath::MAX_LENGTH
    )]
    TooLong,
    #[error("resource path should be an absolute canonical path in an archive e.g. 'base\\mod\\character.ent'")]
    NotCanonical,
}
