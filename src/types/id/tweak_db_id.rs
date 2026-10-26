use std::fmt::Debug;
use std::hash::Hash;

use const_crc32::{crc32, crc32_seed};

use crate::raw::root::RED4ext as red;

/// Tweak database identifier.
/// Creating an ID hashes a name; it does not create or look up a database record.
/// Equality, ordering, and hashing use the CRC32 hash and byte length, ignoring
/// the database offset. Raw `u64` conversions preserve the offset.
///
/// SDK: [TweakDBID](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/NativeTypes.hpp).
#[derive(Default, Clone, Copy)]
#[repr(transparent)]
pub struct TweakDbId(red::TweakDBID);

impl TweakDbId {
    #[inline]
    const fn new_with(hash: u32, length: u8, offset: [u8; 3]) -> Self {
        Self(red::TweakDBID {
            __bindgen_anon_1: red::TweakDBID__bindgen_ty_1 {
                name: red::TweakDBID__bindgen_ty_1__bindgen_ty_1 {
                    hash,
                    length,
                    tdbOffsetBE: offset,
                },
            },
        })
    }

    /// Hashes all UTF-8 bytes of a record name and sets the database offset to zero.
    ///
    /// # Panics
    /// Panics if the name exceeds 255 bytes.
    #[inline]
    pub const fn new(str: &str) -> Self {
        assert!(str.len() <= u8::MAX as usize);
        Self::new_with(crc32(str.as_bytes()), str.len() as u8, [0, 0, 0])
    }

    /// Appends text to a base name's hash and length, resetting the offset to zero.
    /// No separator is inserted; include one in `str` if needed.
    ///
    /// # Panics
    /// Panics if the combined name length exceeds 255 bytes.
    #[inline]
    pub const fn new_from_base(base: TweakDbId, str: &str) -> Self {
        let base_hash = unsafe { base.0.__bindgen_anon_1.name.hash };
        let base_length = unsafe { base.0.__bindgen_anon_1.name.length };
        assert!((base_length as usize + str.len()) <= u8::MAX as usize);
        Self::new_with(
            crc32_seed(str.as_bytes(), base_hash),
            str.len() as u8 + base_length,
            [0, 0, 0],
        )
    }

    /// Returns whether both the name hash and length are nonzero.
    /// Does not check whether a database record exists.
    pub fn is_valid(self) -> bool {
        unsafe { self.0.IsValid() }
    }

    /// Returns whether the stored database offset is nonzero.
    pub fn has_tdb_offset(self) -> bool {
        self.tdb_offset() != 0
    }

    /// Returns the unsigned 24-bit database offset as an `i32`.
    pub fn tdb_offset(self) -> i32 {
        let [b1, b2, b3] = unsafe { self.0.__bindgen_anon_1.name }.tdbOffsetBE;
        i32::from_be_bytes([0, b1, b2, b3])
    }

    /// Returns this ID with an unsigned 24-bit database offset, preserving identity.
    ///
    /// # Panics
    /// Panics if `offset` is outside `0..=0xFF_FFFF`.
    pub fn with_tdb_offset(self, offset: i32) -> Self {
        assert!(
            (0..=0xFF_FFFF).contains(&offset),
            "TDB offset must fit in 24 unsigned bits"
        );
        let [_, b1, b2, b3] = offset.to_be_bytes();
        Self::new_with(
            unsafe { self.0.__bindgen_anon_1.name }.hash,
            unsafe { self.0.__bindgen_anon_1.name }.length,
            [b1, b2, b3],
        )
    }

    pub(super) const fn hash(&self) -> u32 {
        unsafe { self.0.__bindgen_anon_1.name }.hash
    }

    pub(super) const fn len(&self) -> u8 {
        unsafe { self.0.__bindgen_anon_1.name }.length
    }

    pub(super) const fn to_inner(self) -> red::TweakDBID {
        self.0
    }
}

impl Debug for TweakDbId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TweakDbId")
            .field(&unsafe { self.0.__bindgen_anon_1.value })
            .finish()
    }
}

impl PartialEq for TweakDbId {
    fn eq(&self, other: &Self) -> bool {
        self.hash() == other.hash() && self.len() == other.len()
    }
}

impl Eq for TweakDbId {}

impl PartialOrd for TweakDbId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TweakDbId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.hash(), self.len()).cmp(&(other.hash(), other.len()))
    }
}

impl From<u64> for TweakDbId {
    fn from(value: u64) -> Self {
        Self(red::TweakDBID {
            __bindgen_anon_1: red::TweakDBID__bindgen_ty_1 { value },
        })
    }
}

impl From<TweakDbId> for u64 {
    fn from(value: TweakDbId) -> Self {
        unsafe { value.0.__bindgen_anon_1.value }
    }
}

impl Hash for TweakDbId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash().hash(state);
        self.len().hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::TweakDbId;

    #[test]
    fn sdk_layout() {
        assert_eq!(std::mem::size_of::<TweakDbId>(), 8);
        assert_eq!(std::mem::align_of::<TweakDbId>(), 1);
        assert_eq!(
            std::mem::align_of::<TweakDbId>(),
            std::mem::align_of::<crate::raw::root::RED4ext::TweakDBID>()
        );
    }

    #[test]
    fn conversion() {
        assert_eq!(
            TweakDbId::new("Items.FirstAidWhiffV0"),
            TweakDbId::from(90_628_141_458)
        );
        assert_eq!(
            u64::from(TweakDbId::new("Items.FirstAidWhiffV0")),
            90_628_141_458
        );
    }

    #[test]
    fn identity_ignores_offset() {
        use std::collections::{BTreeSet, HashSet};

        let original = TweakDbId::new("Items.FirstAidWhiffV0");
        let modified = original.with_tdb_offset(128);
        assert_ne!(u64::from(original), u64::from(modified));
        assert_eq!(original, modified);
        assert_eq!(HashSet::from([original, modified]).len(), 1);
        assert_eq!(BTreeSet::from([original, modified]).len(), 1);
        // Hash takes precedence over length, unlike the packed u64 ordering.
        assert!(TweakDbId::new_with(1, 255, [0; 3]) < TweakDbId::new_with(2, 0, [0; 3]));
        assert!(TweakDbId::new_with(1, 1, [0; 3]) < TweakDbId::new_with(1, 2, [0; 3]));
    }

    #[test]
    fn offset_bounds() {
        let id = TweakDbId::new("test");
        for offset in [0, 1, 0x7F_FFFF, 0x80_0000, 0xFF_FFFF] {
            assert_eq!(id.with_tdb_offset(offset).tdb_offset(), offset);
        }
        for offset in [-1, i32::MIN, 0x100_0000, i32::MAX] {
            assert!(std::panic::catch_unwind(|| id.with_tdb_offset(offset)).is_err());
        }
    }

    #[test]
    fn mutation() {
        let original = TweakDbId::from(90_628_141_458);
        let modified = original.with_tdb_offset(128);
        assert_eq!(original.tdb_offset(), 0);
        assert_eq!(modified.tdb_offset(), 128);
    }
}
