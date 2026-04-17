use std::{fmt, mem, str};

use crate::raw::root::RED4ext as red;
use crate::types::RedString;

#[repr(transparent)]
pub struct LocalizationString(red::LocalizationString);

impl LocalizationString {
    #[inline]
    pub fn len(&self) -> usize {
        if self.0.unk00 == 0 {
            return self.0.unk08.length as usize;
        }
        str::from_utf8(&self.0.unk00.to_ne_bytes())
            .unwrap_or("")
            .len()
            + self.0.unk08.length as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl fmt::Display for LocalizationString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.unk00 == 0 {
            return write!(f, "{}", unsafe {
                mem::transmute::<&red::CString, &RedString>(&self.0.unk08)
            });
        }
        write!(
            f,
            "{}{}",
            str::from_utf8(&self.0.unk00.to_ne_bytes()).unwrap_or(""),
            unsafe { mem::transmute::<&red::CString, &RedString>(&self.0.unk08) }
        )
    }
}
