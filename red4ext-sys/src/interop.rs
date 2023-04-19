use std::ffi::CStr;
use std::ptr;

use cxx::{type_id, ExternType};

use crate::ffi;

pub type Mem = *mut std::ffi::c_void;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[repr(C)]
pub struct CName {
    hash: u64,
}

impl CName {
    #[inline]
    pub const fn new(str: &str) -> Self {
        Self { hash: fnv1a64(str) }
    }
}

#[inline]
pub const fn fnv1a64(str: &str) -> u64 {
    const PRIME: u64 = 0x0100_0000_01b3;
    const SEED: u64 = 0xCBF2_9CE4_8422_2325;

    let mut tail = str.as_bytes();
    let mut hash = SEED;
    loop {
        match tail.split_first() {
            Some((head, rem)) => {
                hash ^= *head as u64;
                hash = hash.wrapping_mul(PRIME);
                tail = rem;
            }
            None => break hash,
        }
    }
}

unsafe impl ExternType for CName {
    type Id = type_id!("RED4ext::CName");
    type Kind = cxx::kind::Trivial;
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct REDString {
    data: [i8; 0x14],
    length: u32,
    _allocator: Mem,
}

impl REDString {
    pub fn as_str(&self) -> &str {
        unsafe {
            let ptr = if self.length < 0x4000_0000 {
                self.data.as_ptr()
            } else {
                *(self.data.as_ptr() as *const *const i8)
            };
            CStr::from_ptr(ptr).to_str().unwrap()
        }
    }
}

impl Default for REDString {
    #[inline]
    fn default() -> Self {
        Self {
            data: [0; 0x14],
            length: 0,
            _allocator: ptr::null_mut(),
        }
    }
}

unsafe impl ExternType for REDString {
    type Id = type_id!("RED4ext::CString");
    type Kind = cxx::kind::Trivial;
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Variant {
    typ: *const ffi::CBaseRTTIType,
    data: [u8; 0x10],
}

impl Variant {
    #[inline]
    pub const fn undefined() -> Self {
        Variant {
            typ: ptr::null(),
            data: [0; 0x10],
        }
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

#[repr(u8)]
pub enum MainReason {
    Load = 0,
    Unload = 1,
}

unsafe impl ExternType for MainReason {
    type Id = type_id!("RED4ext::EMainReason");
    type Kind = cxx::kind::Trivial;
}

#[derive(Debug)]
#[repr(C)]
pub struct StackArg {
    typ: *const ffi::CBaseRTTIType,
    value: Mem,
}

impl StackArg {
    #[inline]
    pub fn new(typ: *const ffi::CBaseRTTIType, value: Mem) -> StackArg {
        StackArg { typ, value }
    }
}

unsafe impl ExternType for StackArg {
    type Id = type_id!("RED4ext::CStackType");
    type Kind = cxx::kind::Trivial;
}

pub struct VoidPtr(pub *mut std::ffi::c_void);

unsafe impl ExternType for VoidPtr {
    type Id = type_id!("glue::VoidPtr");
    type Kind = cxx::kind::Trivial;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_hashes() {
        assert_eq!(CName::new("IScriptable").hash, 3_191_163_302_135_919_211);
        assert_eq!(CName::new("Vector2").hash, 7_466_804_955_052_523_504);
        assert_eq!(CName::new("Color").hash, 3_769_135_706_557_701_272);
    }
}
