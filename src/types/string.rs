use std::ffi::{CStr, CString};
use std::{fmt, ops, ptr};

use crate::raw::root::RED4ext as red;

#[repr(transparent)]
pub struct RedString(red::CString);

impl RedString {
    #[inline]
    pub fn new() -> Self {
        Self(unsafe { red::CString::new(ptr::null_mut()) })
    }
}

impl Default for RedString {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for RedString {
    #[inline]
    fn clone(&self) -> Self {
        Self::from(self.as_ref())
    }
}

impl ops::Deref for RedString {
    type Target = CStr;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { CStr::from_ptr(self.0.c_str()) }
    }
}

impl From<&CStr> for RedString {
    #[inline]
    fn from(value: &CStr) -> Self {
        Self(unsafe { red::CString::new1(value.as_ptr(), ptr::null_mut()) })
    }
}

impl From<CString> for RedString {
    #[inline]
    fn from(value: CString) -> Self {
        value.as_c_str().into()
    }
}

impl From<String> for RedString {
    #[inline]
    fn from(value: String) -> Self {
        RedString::from(crate::truncated_cstring(value))
    }
}

impl From<RedString> for String {
    #[inline]
    fn from(value: RedString) -> Self {
        value.to_string_lossy().into_owned()
    }
}

impl AsRef<CStr> for RedString {
    #[inline]
    fn as_ref(&self) -> &CStr {
        self
    }
}

impl fmt::Debug for RedString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_ref())
    }
}

impl fmt::Display for RedString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_lossy())
    }
}

impl Drop for RedString {
    #[inline]
    fn drop(&mut self) {
        unsafe { self.0.destruct() }
    }
}
