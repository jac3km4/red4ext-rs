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
        self.0.unk08.length as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl fmt::Display for LocalizationString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", unsafe {
            mem::transmute::<&red::CString, &RedString>(&self.0.unk08)
        })
    }
}
