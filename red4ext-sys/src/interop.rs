use std::ffi::CStr;
use std::{fmt, pin, ptr};

use cxx::{type_id, ExternType};
pub use ffi::EMainReason;
use red4ext_types::Mem;

use crate::ffi;

pub trait CNameExt {
    fn new_pooled(str: &str) -> Self;
    fn is_valid(&self) -> bool;
    fn display(&self) -> impl fmt::Display;
}

#[cfg(not(test))] // only available in-game
impl CNameExt for red4ext_types::CName {
    fn new_pooled(str: &str) -> Self {
        let cname = Self::new(str);
        if cname.is_valid() {
            return cname;
        }
        let created = crate::ffi::add_cname(str);
        assert_eq!(created, cname);
        created
    }

    fn is_valid(&self) -> bool {
        !crate::ffi::resolve_cname(self).is_empty()
    }

    fn display(&self) -> impl fmt::Display {
        DisplayCName(*self)
    }
}

#[cfg(not(test))]
struct DisplayCName(red4ext_types::CName);

#[cfg(not(test))]
impl std::fmt::Display for DisplayCName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", crate::ffi::resolve_cname(&self.0))
    }
}

/// A string type used in the game. Corresponds to `String` in redscript.
#[derive(Debug)]
#[repr(C, packed)]
pub struct RedString {
    data: [i8; 0x14],
    length: u32,
    _allocator: Mem,
}

impl RedString {
    /// Allocates a new string with the given contents.
    pub fn new(str: impl AsRef<str>) -> Self {
        let mut repr = RedString::default();
        unsafe { ffi::construct_string_at(&mut repr, str.as_ref(), ptr::null_mut()) };
        repr
    }

    /// Retrieves the contents of the string as a slice.
    pub fn as_str(&self) -> &str {
        unsafe {
            let ptr = if self.length < 0x4000_0000 {
                self.data.as_ptr()
            } else {
                *(self.data.as_ptr() as *const _)
            };
            CStr::from_ptr(ptr).to_str().unwrap()
        }
    }
}

impl Drop for RedString {
    fn drop(&mut self) {
        unsafe { ffi::destruct_string(self) }
    }
}

impl Default for RedString {
    #[inline]
    fn default() -> Self {
        Self {
            data: [0; 0x14],
            length: 0,
            _allocator: ptr::null_mut(),
        }
    }
}

impl AsRef<str> for RedString {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for RedString {
    #[inline]
    fn from(str: &str) -> Self {
        Self::new(str)
    }
}

impl From<RedString> for String {
    fn from(s: RedString) -> Self {
        s.as_str().to_owned()
    }
}

unsafe impl ExternType for RedString {
    type Id = type_id!("RED4ext::CString");
    type Kind = cxx::kind::Trivial;
}

/// A union type that can hold any type used in the game. Corresponds to `Variant` in redscript.
#[derive(Debug)]
#[repr(C)]
pub struct Variant {
    typ: *const ffi::CBaseRttiType,
    data: [u8; 0x10],
}

impl Variant {
    /// Creates an undefined variant, which holds no value.
    #[inline]
    pub const fn undefined() -> Self {
        Variant {
            typ: ptr::null(),
            data: [0; 0x10],
        }
    }
}

impl Drop for Variant {
    fn drop(&mut self) {
        unsafe { pin::Pin::new_unchecked(self) }.free();
    }
}

impl Default for Variant {
    #[inline]
    fn default() -> Self {
        Self::undefined()
    }
}

unsafe impl ExternType for Variant {
    type Id = type_id!("RED4ext::Variant");
    type Kind = cxx::kind::Trivial;
}

#[derive(Debug)]
#[repr(C)]
pub struct StackArg {
    typ: *const ffi::CBaseRttiType,
    value: Mem,
}

impl StackArg {
    #[inline]
    pub fn new(typ: *const ffi::CBaseRttiType, value: Mem) -> StackArg {
        StackArg { typ, value }
    }

    pub fn inner_type(&self) -> *const ffi::CBaseRttiType {
        self.typ
    }
}

unsafe impl ExternType for StackArg {
    type Id = type_id!("RED4ext::CStackType");
    type Kind = cxx::kind::Trivial;
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct RefCount {
    strong_refs: u32,
    weak_refs: u32,
}

unsafe impl ExternType for RefCount {
    type Id = type_id!("RED4ext::RefCnt");
    type Kind = cxx::kind::Trivial;
}
