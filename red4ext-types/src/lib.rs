use std::path::Path;

use const_crc32::{crc32, crc32_seed};
use cxx::{type_id, ExternType};
use num_enum::TryFromPrimitive;
use thiserror::Error;

pub type Mem = *mut std::ffi::c_void;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct CName {
    hash: u64,
}

impl From<u64> for CName {
    fn from(hash: u64) -> Self {
        Self { hash }
    }
}

impl From<CName> for u64 {
    fn from(value: CName) -> Self {
        value.hash
    }
}

impl CName {
    #[inline]
    pub const fn new(str: &str) -> Self {
        match str.as_bytes() {
            b"None" => Self { hash: 0 },
            _ => Self { hash: fnv1a64(str) },
        }
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
pub(crate) struct RaRef(ResourcePath);

impl RaRef {
    fn new(path: &str) -> Result<Self, ResourcePathError> {
        Ok(Self(ResourcePath::new(path)?))
    }
}

unsafe impl ExternType for RaRef {
    type Id = type_id!("RED4ext::RaRef");
    type Kind = cxx::kind::Trivial;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct ResourcePath {
    hash: u64,
}

impl ResourcePath {
    pub const MAX_LENGTH: usize = 216;

    /// accepts non-sanitized path of any length,
    /// but final sanitized path length must be equals or inferior to 216 bytes
    fn new(path: &str) -> Result<Self, ResourcePathError> {
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
            return Err(ResourcePathError::TooLong);
        }
        if Path::new(&sanitized)
            .components()
            .any(|x| !matches!(x, std::path::Component::Normal(_)))
        {
            return Err(ResourcePathError::NotCanonical);
        }
        Ok(Self {
            hash: fnv1a64(&sanitized),
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct TweakDbId {
    hash: u32,
    length: u8,
}

impl From<u64> for TweakDbId {
    fn from(value: u64) -> Self {
        Self::from_u64(value)
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

    pub const fn to_u64(self) -> u64 {
        let [b1, b2, b3, b4] = self.hash.to_ne_bytes();
        u64::from_ne_bytes([b1, b2, b3, b4, self.length, 0, 0, 0])
    }

    #[doc(hidden)]
    pub const fn from_u64(value: u64) -> Self {
        let [b1, b2, b3, b4, length, ..] = value.to_ne_bytes();
        let hash = u32::from_ne_bytes([b1, b2, b3, b4]);
        Self { hash, length }
    }
}

unsafe impl ExternType for TweakDbId {
    type Id = type_id!("RED4ext::TweakDBID");
    type Kind = cxx::kind::Trivial;
}

/// see [its C++ representation](https://github.com/WopsS/RED4ext.SDK/blob/master/include/RED4ext/NativeTypes.hpp#L105)
///
/// CET has a [different naming convention for the last two fields](https://wiki.redmodding.org/cyber-engine-tweaks/functions/special-types#toitemid).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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
    #[inline]
    pub const fn new_from(id: TweakDbId) -> Self {
        Self {
            id,
            seed: Seed(DEFAULT_ITEM_ID_RNG_SEED),
            counter: 0,
            structure: GamedataItemStructure::BlueprintStackable as u8,
            flags: GameEItemIdFlag::None as u8,
        }
    }

    pub fn structure(&self) -> Option<GamedataItemStructure> {
        self.structure.try_into().ok()
    }

    pub fn flags(&self) -> Option<GameEItemIdFlag> {
        self.flags.try_into().ok()
    }

    #[inline]
    pub const fn get_tdbid(&self) -> TweakDbId {
        self.id
    }

    #[inline]
    pub const fn is_of_tdbid(&self, tdbid: TweakDbId) -> bool {
        self.id.hash == tdbid.hash && self.id.length == tdbid.length
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
/// and [CET initialization](https://github.com/maximegmd/CyberEngineTweaks/blob/v1.27.1/src/scripting/Scripting.cpp#L311).
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum GameEItemIdFlag {
    #[default]
    None = 0,
    Preview = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seed(u32);

const DEFAULT_ITEM_ID_RNG_SEED: u32 = 2;

impl Default for Seed {
    /// see [CET initialization](https://github.com/maximegmd/CyberEngineTweaks/blob/v1.27.1/src/scripting/Scripting.cpp#L311)
    fn default() -> Self {
        Self(DEFAULT_ITEM_ID_RNG_SEED)
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

impl From<EntityId> for u64 {
    fn from(value: EntityId) -> Self {
        value.hash
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

#[derive(Debug)]
#[repr(transparent)]
pub struct VoidPtr(pub *mut std::ffi::c_void);

unsafe impl ExternType for VoidPtr {
    type Id = type_id!("glue::VoidPtr");
    type Kind = cxx::kind::Trivial;
}

#[derive(Debug, Error)]
pub enum ResourcePathError {
    #[error("resource path should not be empty")]
    Empty,
    #[error(
        "resource path should be less than {} characters",
        ResourcePath::MAX_LENGTH
    )]
    TooLong,
    #[error("resource path should be an absolute canonical path in an archive e.g. 'base\\mod\\character.ent'")]
    NotCanonical,
}

/// shortcut for ResRef creation.
#[macro_export]
macro_rules! res_ref {
    ($base:expr, /$lit:literal $($tt:tt)*) => {
        $crate::res_ref!($base.join($lit), $($tt)*)
    };
    ($base:expr, ) => {
        $base
    };
    ($lit:literal $($tt:tt)*) => {
        $crate::ResRef::new(
            &$crate::res_ref!(::std::path::Path::new($lit), $($tt)*).to_string_lossy()
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_hashes() {
        assert_eq!(CName::new("IScriptable").hash, 3_191_163_302_135_919_211);
        assert_eq!(CName::new("Vector2").hash, 7_466_804_955_052_523_504);
        assert_eq!(CName::new("Color").hash, 3_769_135_706_557_701_272);
        assert_eq!(CName::new("None").hash, 0);
        assert_eq!(CName::new("").hash, 0xCBF2_9CE4_8422_2325);
    }

    #[test]
    fn ids() {
        assert_eq!(
            TweakDbId::new("Items.FirstAidWhiffV0"),
            TweakDbId::from(90_628_141_458)
        );
        assert_eq!(
            TweakDbId::new("Items.FirstAidWhiffV0").to_u64(),
            90_628_141_458
        );
        assert_eq!(
            ItemId::new_from(TweakDbId::new("Items.FirstAidWhiffV0")).get_tdbid(),
            TweakDbId::new("Items.FirstAidWhiffV0")
        );
        assert!(ItemId::new_from(TweakDbId::new("Items.FirstAidWhiffV0"))
            .is_of_tdbid(TweakDbId::new("Items.FirstAidWhiffV0")),);
        assert!(!ItemId::new_from(TweakDbId::new("Items.FirstAidWhiffV0"))
            .is_of_tdbid(TweakDbId::new("Items.FirstAidWhiffV1")),);
    }

    #[test]
    fn resource_path() {
        assert_eq!(ResourcePath::default(), ResourcePath { hash: 0 });

        const TOO_LONG: &str = "base\\some\\archive\\path\\that\\is\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\very\\long\\and\\above\\216\\bytes";
        assert!(TOO_LONG.as_bytes().len() > ResourcePath::MAX_LENGTH);
        assert!(ResourcePath::new(TOO_LONG).is_err());

        assert_eq!(
            ResourcePath::new("\'base/somewhere/in/archive/\'").unwrap(),
            ResourcePath {
                hash: fnv1a64("base\\somewhere\\in\\archive")
            }
        );
        assert_eq!(
            ResourcePath::new("\"MULTI\\\\SOMEWHERE\\\\IN\\\\ARCHIVE\"").unwrap(),
            ResourcePath {
                hash: fnv1a64("multi\\somewhere\\in\\archive")
            }
        );
        assert!(ResourcePath::new("..\\somewhere\\in\\archive\\custom.ent").is_err());
        assert!(ResourcePath::new("base\\somewhere\\in\\archive\\custom.ent").is_ok());
        assert!(ResourcePath::new("custom.ent").is_ok());
        assert!(ResourcePath::new(".custom.ent").is_ok());
    }

    #[test]
    fn res_path() {
        use crate::res_ref;
        assert!(res_ref!("").is_err());
        assert!(res_ref!(".." / "somewhere" / "in" / "archive" / "custom.ent").is_err());
        assert!(res_ref!("base" / "somewhere" / "in" / "archive" / "custom.ent").is_ok());
        assert!(res_ref!("custom.ent").is_ok());
        assert!(res_ref!(".custom.ent").is_ok());
    }
}
