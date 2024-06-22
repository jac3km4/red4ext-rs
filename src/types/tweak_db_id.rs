use std::fmt::Debug;
use std::hash::Hash;

use const_crc32::{crc32, crc32_seed};

use crate::raw::root::RED4ext as red;

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

    #[inline]
    pub const fn new(str: &str) -> Self {
        assert!(str.len() <= u8::MAX as usize);
        Self::new_with(crc32(str.as_bytes()), str.len() as u8, [0, 0, 0])
    }

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

    pub fn is_valid(self) -> bool {
        unsafe { self.0.IsValid() }
    }

    pub fn has_tdb_offset(self) -> bool {
        self.tdb_offset() != 0
    }

    pub fn tdb_offset(self) -> i32 {
        let [b1, b2, b3] = unsafe { self.0.__bindgen_anon_1.name }.tdbOffsetBE;
        i32::from_be_bytes([0, b1, b2, b3])
    }

    pub fn with_tdb_offset(self, offset: i32) -> Self {
        assert!(offset <= (i8::MAX as i32 * i8::MAX as i32 * i8::MAX as i32));
        assert!(offset >= (i8::MIN as i32 * i8::MIN as i32 * i8::MIN as i32));
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
        u64::from(*self).eq(&u64::from(*other))
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
        u64::from(*self).cmp(&u64::from(*other))
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
        u64::from(*self).hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::TweakDbId;

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
    fn mutation() {
        let original = TweakDbId::from(90_628_141_458);
        let modified = original.with_tdb_offset(128);
        assert_eq!(original.tdb_offset(), 0);
        assert_eq!(modified.tdb_offset(), 128);
    }
}
