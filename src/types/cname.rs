use std::ffi::{self, CStr};
use std::fmt;
use std::hash::Hash;

use crate::fnv1a64;
use crate::raw::root::RED4ext as red;

/// Hashed name.
/// Creating a hash does not register its text in [`CNamePool`]. Equality and
/// ordering compare hashes, not text. The default value is undefined (zero).
///
/// SDK: [CName](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/CName.hpp).
#[derive(Debug, Default, Clone, Copy)]
#[repr(transparent)]
pub struct CName(red::CName);

impl CName {
    /// Computes FNV-1a without engine access; `"None"` and empty text map to zero.
    /// Use [`CNamePool::add_cstr`] to register text for reverse lookup.
    #[inline]
    pub const fn new(name: &str) -> Self {
        Self::from_bytes(name.as_bytes())
    }

    /// Hashes bytes without registering them; empty bytes and `b"None"` map to zero.
    /// All bytes are included, including embedded NULs.
    pub const fn from_bytes(name: &[u8]) -> Self {
        #[allow(clippy::equatable_if_let)]
        if let b"" | b"None" = name {
            return Self::undefined();
        }
        Self(red::CName {
            hash: fnv1a64(name),
        })
    }

    /// Returns a [`CName`] representing an undefined name.
    #[inline]
    pub const fn undefined() -> Self {
        Self(red::CName { hash: 0 })
    }

    pub(super) fn from_raw(raw: red::CName) -> Self {
        Self(raw)
    }

    pub(super) fn to_raw(self) -> red::CName {
        self.0
    }

    /// Looks up text in the engine name pool; requires an initialized engine.
    ///
    /// # Panics
    /// Panics if the returned text is not UTF-8.
    pub fn as_str(&self) -> &'static str {
        unsafe { ffi::CStr::from_ptr(self.0.ToString()) }
            .to_str()
            .unwrap()
    }
}

impl From<u64> for CName {
    fn from(hash: u64) -> Self {
        Self(red::CName { hash })
    }
}

impl From<CName> for u64 {
    fn from(CName(red::CName { hash }): CName) -> Self {
        hash
    }
}

impl std::fmt::Display for CName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            if self.0.hash == 0 {
                "None"
            } else {
                self.as_str()
            }
        )
    }
}

impl PartialEq for CName {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0.hash == other.0.hash
    }
}

impl Eq for CName {}

impl PartialOrd for CName {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CName {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.hash.cmp(&other.0.hash)
    }
}

impl Hash for CName {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash.hash(state)
    }
}

/// Pool for registered name text. Hashes made with [`CName::new`] are not added automatically.
///
/// SDK: [CNamePool](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/CNamePool.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct CNamePool(red::CNamePool);

impl CNamePool {
    /// Registers text for reverse lookup and returns its name hash.
    /// Requires the engine name pool to be initialized.
    pub fn add_cstr(str: &CStr) -> CName {
        unsafe {
            let add_cstr = crate::fn_from_hash!(
                CNamePool_AddCstr,
                unsafe extern "C" fn(&mut CName, *const i8)
            );
            let mut cname = CName::default();
            add_cstr(&mut cname, str.as_ptr());
            cname
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn calculate_hashes() {
        assert_eq!(
            u64::from(CName::new("IScriptable")),
            3_191_163_302_135_919_211
        );
        assert_eq!(u64::from(CName::new("Vector2")), 7_466_804_955_052_523_504);
        assert_eq!(u64::from(CName::new("Color")), 3_769_135_706_557_701_272);
        assert_eq!(u64::from(CName::new("None")), 0);
        assert_eq!(u64::from(CName::new("")), 0);
        assert_eq!(CName::from_bytes(b""), CName::undefined());
    }
}
