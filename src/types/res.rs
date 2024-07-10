use std::hash::Hash;
use std::marker::PhantomData;
use std::path::Path;

use thiserror::Error;

use crate::fnv1a64;
use crate::raw::root::RED4ext as red;

pub const MAX_LENGTH: usize = 216;

#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct RaRef<T>(red::RaRef, PhantomData<T>);

impl<T> RaRef<T> {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, ResourcePathError> {
        Ok(Self(
            red::RaRef {
                path: red::ResourcePath {
                    hash: encode_path(path)?,
                },
            },
            PhantomData,
        ))
    }
}

impl<T> PartialEq for RaRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.path.hash.eq(&other.0.path.hash)
    }
}

impl<T> Eq for RaRef<T> {}

impl<T> PartialOrd for RaRef<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for RaRef<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.path.hash.cmp(&other.0.path.hash)
    }
}

impl<T> Hash for RaRef<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.path.hash.hash(state);
    }
}

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct ResRef(red::ResRef);

impl ResRef {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, ResourcePathError> {
        Ok(Self(red::ResRef {
            resource: red::RaRef {
                path: red::ResourcePath {
                    hash: encode_path(path)?,
                },
            },
        }))
    }
}

impl PartialEq for ResRef {
    fn eq(&self, other: &Self) -> bool {
        self.0.resource.path.hash.eq(&other.0.resource.path.hash)
    }
}

impl Eq for ResRef {}

impl PartialOrd for ResRef {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ResRef {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.resource.path.hash.cmp(&other.0.resource.path.hash)
    }
}

impl Hash for ResRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.resource.path.hash.hash(state);
    }
}

impl Clone for ResRef {
    fn clone(&self) -> Self {
        Self(red::ResRef {
            resource: self.0.resource,
        })
    }
}

fn encode_path(path: impl AsRef<Path>) -> Result<u64, ResourcePathError> {
    let sanitized = path
        .as_ref()
        .to_str()
        .ok_or(ResourcePathError::InvalidUnicode)?;
    let sanitized = sanitized
        .trim_start_matches(['\'', '\"'])
        .trim_end_matches(['\'', '\"'])
        .trim_start_matches(['/', '\\'])
        .trim_end_matches(['/', '\\'])
        .split(['/', '\\'])
        .filter(|comp| !comp.is_empty())
        .map(str::to_ascii_lowercase)
        .reduce(|mut acc, e| {
            acc.push('\\');
            acc.push_str(&e);
            acc
        })
        .ok_or(ResourcePathError::Empty)?;
    if sanitized.as_bytes().len() > self::MAX_LENGTH {
        return Err(ResourcePathError::TooLong);
    }
    if Path::new(&sanitized)
        .components()
        .any(|x| !matches!(x, std::path::Component::Normal(_)))
    {
        return Err(ResourcePathError::NotCanonical);
    }
    Ok(fnv1a64(&sanitized))
}

#[derive(Debug, Error)]
pub enum ResourcePathError {
    #[error("resource path should not be empty")]
    Empty,
    #[error("resource path should be less than {} characters", self::MAX_LENGTH)]
    TooLong,
    #[error("resource path should be an absolute canonical path in an archive e.g. 'base\\mod\\character.ent'")]
    NotCanonical,
    #[error("resource path should be valid UTF-8")]
    InvalidUnicode,
}

/// shortcut for ResRef creation.
#[macro_export]
macro_rules! res_ref {
    ($base:expr, /$lit:literal $($tt:tt)*) => {
        $crate::res_ref!($base.join($lit), $($tt)*)
    };
    ($base:expr, ) => {
        $base
    };
    ($lit:literal $($tt:tt)*) => {
        $crate::types::ResRef::new(
            $crate::res_ref!(::std::path::Path::new($lit), $($tt)*)
        )
    };
}

#[cfg(test)]
mod tests {
    use super::{encode_path, ResRef};
    use crate::fnv1a64;

    #[test]
    fn resource_path() {
        const TOO_LONG: &str = "base\\some\\archive\\path\\that\\is\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\long\\and\\above\\216\\bytes";
        assert!(TOO_LONG.as_bytes().len() > super::MAX_LENGTH);
        assert!(encode_path(TOO_LONG).is_err());

        assert_eq!(
            encode_path("\'base/somewhere/in/archive/\'").unwrap(),
            fnv1a64("base\\somewhere\\in\\archive")
        );
        assert_eq!(
            encode_path("\"MULTI\\\\SOMEWHERE\\\\IN\\\\ARCHIVE\"").unwrap(),
            fnv1a64("multi\\somewhere\\in\\archive")
        );
        assert!(encode_path("..\\somewhere\\in\\archive\\custom.ent").is_err());
        assert!(encode_path("base\\somewhere\\in\\archive\\custom.ent").is_ok());
        assert!(encode_path("custom.ent").is_ok());
        assert!(encode_path(".custom.ent").is_ok());
    }

    #[test]
    fn res_path() {
        assert!(res_ref!("").is_err());
        assert!(res_ref!(".." / "somewhere" / "in" / "archive" / "custom.ent").is_err());
        assert!(res_ref!("base" / "somewhere" / "in" / "archive" / "custom.ent").is_ok());
        assert!(res_ref!("custom.ent").is_ok());
        assert!(res_ref!(".custom.ent").is_ok());

        assert_eq!(
            res_ref!("base" / "somewhere" / "in" / "archive" / "custom.ent").unwrap(),
            ResRef::new(std::path::Path::new(
                "base\\somewhere\\in\\archive\\custom.ent"
            ))
            .unwrap()
        );
        assert_eq!(
            res_ref!("custom.ent").unwrap(),
            ResRef::new(std::path::Path::new("custom.ent")).unwrap()
        );
        assert_eq!(
            res_ref!(".custom.ent").unwrap(),
            ResRef::new(std::path::Path::new(".custom.ent")).unwrap()
        );
    }
}
