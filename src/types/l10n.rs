use std::{fmt, mem};

use crate::raw::root::RED4ext as red;
use crate::types::RedString;

/// SDK [LocalizationString](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/NativeTypes.hpp).
/// Display reads the stored text; it does not perform a localization lookup.
#[repr(transparent)]
pub struct LocalizationString(red::LocalizationString);

impl LocalizationString {
    #[inline]
    pub fn len(&self) -> usize {
        (self.0.unk08.length & 0x3FFF_FFFF) as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::LocalizationString;

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

impl fmt::Display for LocalizationString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", unsafe {
            mem::transmute::<&red::CString, &RedString>(&self.0.unk08)
        })
    }
}
