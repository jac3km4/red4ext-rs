use crate::{raw::root::RED4ext as red, types::RedString};
use std::mem;

#[repr(transparent)]
pub struct LocalizationString(red::LocalizationString);

impl std::fmt::Display for LocalizationString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", unsafe {
            mem::transmute::<&red::CString, &RedString>(&self.0.unk08)
        })
    }
}
