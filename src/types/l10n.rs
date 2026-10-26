use std::{fmt, mem};

use crate::raw::root::RED4ext as red;
use crate::types::RedString;

/// Localization string.
/// Owns its stored text; dropping it requires initialized engine string functions.
/// Display reads that text with lossy UTF-8 decoding, without a localization lookup.
///
/// SDK: [LocalizationString](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/NativeTypes.hpp).
#[repr(transparent)]
pub struct LocalizationString(red::LocalizationString);

impl LocalizationString {
    /// Returns the stored text length in bytes, excluding the terminating NUL.
    #[inline]
    pub fn len(&self) -> usize {
        (self.0.unk08.length & 0x3FFF_FFFF) as usize
    }

    /// Returns whether the stored text has zero length.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Drop for LocalizationString {
    fn drop(&mut self) {
        // The SDK aggregate's implicit destructor destroys its CString member.
        unsafe { self.0.unk08.destruct() }
    }
}

impl fmt::Display for LocalizationString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", unsafe {
            mem::transmute::<&red::CString, &RedString>(&self.0.unk08)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::LocalizationString;

    #[test]
    fn owns_native_string() {
        assert!(std::mem::needs_drop::<LocalizationString>());
    }

    #[test]
    fn length_masks_storage_flags() {
        for flags in [0, 0x4000_0000, 0x8000_0000, 0xC000_0000] {
            // Inspect metadata only; the synthetic CString must never be destroyed.
            let mut value =
                std::mem::ManuallyDrop::new(LocalizationString(unsafe { std::mem::zeroed() }));
            value.0.unk08.length = flags | 17;
            assert_eq!(value.len(), 17);
            value.0.unk08.length = flags;
            assert!(value.is_empty());
        }
    }
}
