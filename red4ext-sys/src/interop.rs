use std::ffi::{CStr, OsStr};
use std::path::PathBuf;
use std::ptr;

use const_crc32::{crc32, crc32_seed};
use cxx::{type_id, ExternType};
pub use ffi::EMainReason;
use num_enum::TryFromPrimitive;

use crate::error::ResourcePathError;
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[repr(C)]
pub struct ResRef(RaRef);

impl ResRef {
    pub fn new(path: &str) -> Result<Self, ResourcePathError> {
        Ok(Self(RaRef::new(path)?))
    }
}

unsafe impl ExternType for ResRef {
    type Id = type_id!("RED4ext::ResRef");
    type Kind = cxx::kind::Trivial;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[repr(C)]
pub struct RaRef(ResourcePath);

impl RaRef {
    fn new(path: &str) -> Result<Self, ResourcePathError> {
        Ok(Self(ResourcePath::try_new(path)?))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct ResourcePath {
    hash: u64,
}

impl ResourcePath {
    const MAX_LENGTH: usize = 216;

    /// accepts non-sanitized path of any length,
    /// but final sanitized path length must be equals or inferior to 216 bytes
    fn try_new(path: &str) -> Result<Self, ResourcePathError> {
        let sanitized = path
            .trim_start_matches(|c| c == '\'' || c == '\"')
            .trim_end_matches(|c| c == '\'' || c == '\"')
            .trim_start_matches(|c| c == '/' || c == '\\')
            .trim_end_matches(|c| c == '/' || c == '\\')
            .split(|c| c == '/' || c == '\\')
            .filter(|comp| !comp.is_empty())
            .map(str::to_ascii_lowercase)
            .reduce(|mut acc, e| {
                acc.push('\\');
                acc.push_str(&e);
                acc
            })
            .ok_or(ResourcePathError::Empty)?;
        if sanitized.as_bytes().len() > Self::MAX_LENGTH {
            return Err(ResourcePathError::TooLong {
                max: ResourcePath::MAX_LENGTH,
            });
        }
        if let Ok(ref segments) = PathBuf::try_from(&sanitized) {
            let only_dots = |x: &OsStr| {
                for (idx, character) in x.to_string_lossy().chars().enumerate() {
                    if idx == 0 && character == '.' {
                        continue;
                    }
                    if character == '.' {
                        continue;
                    }
                    return false;
                }
                true
            };
            if segments.iter().any(only_dots) {
                return Err(ResourcePathError::Relative {
                    path: path.to_string(),
                });
            }
        }
        Ok(Self {
            hash: fnv1a64(&sanitized),
        })
    }

    pub fn builder() -> ResourcePathBuilder {
        ResourcePathBuilder {
            components: PathBuf::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.hash == 0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[repr(C)]
pub struct TweakDbId {
    hash: u32,
    length: u8,
}

impl From<u64> for TweakDbId {
    fn from(value: u64) -> Self {
        let [b1, b2, b3, b4, length, ..] = value.to_ne_bytes();
        let hash = u32::from_ne_bytes([b1, b2, b3, b4]);
        Self { hash, length }
    }
}

impl TweakDbId {
    #[inline]
    pub const fn new(str: &str) -> Self {
        assert!(str.len() <= u8::MAX as usize);
        Self {
            hash: crc32(str.as_bytes()),
            length: str.len() as u8,
        }
    }

    #[inline]
    pub const fn new_from_base(base: TweakDbId, str: &str) -> Self {
        assert!((base.length as usize + str.len()) <= u8::MAX as usize);
        Self {
            hash: crc32_seed(str.as_bytes(), base.hash),
            length: str.len() as u8 + base.length,
        }
    }
}

unsafe impl ExternType for TweakDbId {
    type Id = type_id!("RED4ext::TweakDBID");
    type Kind = cxx::kind::Trivial;
}

/// see [its C++ representation](https://github.com/WopsS/RED4ext.SDK/blob/master/include/RED4ext/NativeTypes.hpp#L105)
///
/// CET has a [different naming convention for the last two fields](https://wiki.redmodding.org/cyber-engine-tweaks/functions/special-types#toitemid).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[repr(C)]
pub struct ItemId {
    id: TweakDbId,
    seed: Seed,
    counter: u16,
    /// also called `unknown` in CET
    structure: u8,
    /// also called `maybe_type` in CET
    flags: u8,
}

impl ItemId {
    pub fn new_from(id: TweakDbId) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

    pub fn structure(&self) -> Option<GamedataItemStructure> {
        self.structure.try_into().ok()
    }

    pub fn flags(&self) -> Option<GameEItemIdFlag> {
        self.flags.try_into().ok()
    }
}

unsafe impl ExternType for ItemId {
    type Id = type_id!("RED4ext::ItemID");
    type Kind = cxx::kind::Trivial;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum GamedataItemStructure {
    #[default]
    BlueprintStackable = 0,
    Stackable = 1,
    Unique = 2,
    Count = 3,
    Invalid = 4,
}

/// see [gameEItemIDFlag](https://nativedb.red4ext.com/gameEItemIDFlag)
/// and [CET initialization](https://github.com/maximegmd/CyberEngineTweaks/blob/v1.24.1/src/scripting/Scripting.cpp#L311).
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum GameEItemIdFlag {
    #[default]
    None = 0,
    Preview = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seed(u32);

impl Default for Seed {
    /// see [CET initialization](https://github.com/maximegmd/CyberEngineTweaks/blob/v1.24.1/src/scripting/Scripting.cpp#L311)
    fn default() -> Self {
        Self(2)
    }
}

impl From<u32> for Seed {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<Seed> for u32 {
    fn from(value: Seed) -> Self {
        value.0
    }
}

/// see [its C++ representation](https://github.com/WopsS/RED4ext.SDK/blob/master/include/RED4ext/Scripting/Natives/entEntityID.hpp#L7)
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EntityId {
    hash: u64,
}

impl From<u64> for EntityId {
    fn from(hash: u64) -> Self {
        Self { hash }
    }
}

impl EntityId {
    const DYNAMIC_UPPER_BOUND: u64 = 0x00FF_FFFF;
    const PERSISTABLE_LOWER_BOUND: u64 = 9_000_000;
    const PERSISTABLE_UPPER_BOUND: u64 = 10_000_000;

    #[inline]
    pub fn is_defined(&self) -> bool {
        self.hash != 0
    }

    #[inline]
    pub fn is_static(&self) -> bool {
        self.hash != 0 && self.hash > Self::DYNAMIC_UPPER_BOUND
    }

    #[inline]
    pub fn is_dynamic(&self) -> bool {
        self.hash != 0 && self.hash <= Self::DYNAMIC_UPPER_BOUND
    }

    #[inline]
    pub fn is_persistable(&self) -> bool {
        self.hash >= Self::PERSISTABLE_LOWER_BOUND && self.hash < Self::PERSISTABLE_UPPER_BOUND
    }
}

unsafe impl ExternType for EntityId {
    type Id = type_id!("RED4ext::EntityID");
    type Kind = cxx::kind::Trivial;
}

#[derive(Debug, Clone)]
#[repr(C, packed)]
pub struct RedString {
    data: [i8; 0x14],
    length: u32,
    _allocator: Mem,
}

impl RedString {
    pub fn new(str: impl AsRef<str>) -> Self {
        let mut repr = RedString::default();
        unsafe { ffi::construct_string_at(&mut repr, str.as_ref(), ptr::null_mut()) };
        repr
    }

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

unsafe impl ExternType for RedString {
    type Id = type_id!("RED4ext::CString");
    type Kind = cxx::kind::Trivial;
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Variant {
    typ: *const ffi::CBaseRttiType,
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

pub struct VoidPtr(pub *mut std::ffi::c_void);

unsafe impl ExternType for VoidPtr {
    type Id = type_id!("glue::VoidPtr");
    type Kind = cxx::kind::Trivial;
}

pub struct ResourcePathBuilder {
    components: PathBuf,
}

impl ResourcePathBuilder {
    pub fn join(mut self, component: impl Into<PathBuf>) -> Self {
        self.components.push(component.into());
        self
    }

    pub fn try_build(self) -> Result<ResourcePath, ResourcePathError> {
        ResourcePath::try_new(&self.components.to_string_lossy())
    }
}

/// shortcut for ResourcePath creation.
#[macro_export]
macro_rules! res_path {
    ($base:expr, /$lit:literal $($tt:tt)*) => {
        $crate::res_path!($base.join($lit), $($tt)*)
    };
    ($base:expr, ) => {
        $base
    };
    ($lit:literal $($tt:tt)*) => {
        $crate::res_path!($crate::interop::ResourcePath::builder().join($lit), $($tt)*).try_build()
    };
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn calculate_hashes() {
        assert_eq!(CName::new("IScriptable").hash, 3_191_163_302_135_919_211);
        assert_eq!(CName::new("Vector2").hash, 7_466_804_955_052_523_504);
        assert_eq!(CName::new("Color").hash, 3_769_135_706_557_701_272);
    }

    #[test]
    fn resource_path() {
        assert_eq!(ResourcePath::default(), ResourcePath { hash: 0 });

        const TOO_LONG: &str = "base\\some\\archive\\path\\that\\is\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\long\\and\\above\\216\\bytes";
        assert!(TOO_LONG.as_bytes().len() > ResourcePath::MAX_LENGTH);
        assert!(ResourcePath::try_new(TOO_LONG).is_err());

        assert_eq!(
            ResourcePath::try_new("\'base/somewhere/in/archive/\'").unwrap(),
            ResourcePath {
                hash: fnv1a64("base\\somewhere\\in\\archive")
            }
        );
        assert_eq!(
            ResourcePath::try_new("\"MULTI\\\\SOMEWHERE\\\\IN\\\\ARCHIVE\"").unwrap(),
            ResourcePath {
                hash: fnv1a64("multi\\somewhere\\in\\archive")
            }
        );
        assert!(ResourcePath::try_new("..\\somewhere\\in\\archive\\custom.ent").is_err());
        assert!(ResourcePath::try_new("base\\somewhere\\in\\archive\\custom.ent").is_ok());
        assert!(ResourcePath::try_new("custom.ent").is_ok());
        assert!(ResourcePath::try_new(".custom.ent").is_ok());
    }

    #[test]
    fn builder() {
        assert_eq!(
            ResourcePath::builder()
                .join("base")
                .join("somewhere")
                .join("in")
                .join("archive")
                .try_build()
                .unwrap(),
            ResourcePath {
                hash: fnv1a64("base\\somewhere\\in\\archive")
            }
        );

        let path = PathBuf::from("multi\\").join("somewhere");
        assert_eq!(
            ResourcePath::builder()
                .join(path)
                .join("in")
                .join("archive")
                .try_build()
                .unwrap(),
            ResourcePath {
                hash: fnv1a64("multi\\somewhere\\in\\archive")
            }
        );
    }
    #[cfg(feature = "macros")]
    #[test]
    fn res_path() {
        use crate::res_path;
        assert!(res_path!("").is_err());
        assert!(res_path!(".." / "somewhere" / "in" / "archive" / "custom.ent").is_err());
        assert!(res_path!("base" / "somewhere" / "in" / "archive" / "custom.ent").is_ok());
        assert!(res_path!("custom.ent").is_ok());
        assert!(res_path!(".custom.ent").is_ok());
    }
}
