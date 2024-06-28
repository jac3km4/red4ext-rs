use std::fmt;

use super::TweakDbId;
use crate::raw::root::RED4ext as red;

const DEFAULT_ITEM_ID_RNG_SEED: u32 = 2;

#[derive(Default, Clone, Copy)]
#[repr(transparent)]
pub struct ItemId(red::ItemID);

impl ItemId {
    #[inline]
    pub const fn new_from(id: TweakDbId) -> Self {
        Self(red::ItemID {
            tdbid: id.to_inner(),
            rngSeed: DEFAULT_ITEM_ID_RNG_SEED,
            uniqueCounter: 0,
            structure: GamedataItemStructure::BlueprintStackable as u8,
            flags: GameEItemIdFlag::None as u8,
        })
    }

    pub fn structure(&self) -> GamedataItemStructure {
        self.0.structure.into()
    }

    pub fn flags(&self) -> GameEItemIdFlag {
        self.0.flags.into()
    }

    #[inline]
    pub fn tdbid(&self) -> TweakDbId {
        TweakDbId::from(unsafe { self.0.tdbid.__bindgen_anon_1.value })
    }

    #[inline]
    pub const fn is_of_tdbid(&self, tdbid: TweakDbId) -> bool {
        unsafe { self.0.tdbid.__bindgen_anon_1.name }.hash == tdbid.hash()
            && unsafe { self.0.tdbid.__bindgen_anon_1.name }.length == tdbid.len()
    }

    pub fn is_valid(&self) -> bool {
        unsafe { self.0.IsValid() }
    }
}

impl fmt::Debug for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ItemId")
            .field("tdbid", &self.tdbid())
            .field("structure", &self.structure())
            .field("flags", &self.flags())
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Seed(u32);

impl Default for Seed {
    fn default() -> Self {
        Self(DEFAULT_ITEM_ID_RNG_SEED)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq)]
#[repr(u8)]
pub enum GamedataItemStructure {
    #[default]
    BlueprintStackable = 0,
    Stackable = 1,
    Unique = 2,
    Count = 3,
    Invalid = 4,
}

impl From<u8> for GamedataItemStructure {
    fn from(value: u8) -> Self {
        match value {
            v if v == Self::BlueprintStackable as u8 => Self::BlueprintStackable,
            v if v == Self::Stackable as u8 => Self::Stackable,
            v if v == Self::Unique as u8 => Self::Unique,
            v if v == Self::Count as u8 => Self::Count,
            _ => Self::Invalid,
        }
    }
}

/// see [gameEItemIDFlag](https://nativedb.red4ext.com/gameEItemIDFlag)
/// and [CET initialization](https://github.com/maximegmd/CyberEngineTweaks/blob/v1.27.1/src/scripting/Scripting.cpp#L311).
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq)]
#[repr(u8)]
pub enum GameEItemIdFlag {
    #[default]
    None = 0,
    Preview = 1,
}

impl From<u8> for GameEItemIdFlag {
    fn from(value: u8) -> Self {
        match value {
            v if v == Self::Preview as u8 => Self::Preview,
            _ => Self::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Not;

    use super::{ItemId, TweakDbId};

    const V0: TweakDbId = TweakDbId::new("Items.FirstAidWhiffV0");
    const V1: TweakDbId = TweakDbId::new("Items.FirstAidWhiffV1");

    #[test]
    fn comparison() {
        assert_eq!(ItemId::new_from(V0).tdbid(), V0);
        assert!(ItemId::new_from(V0).is_of_tdbid(V0));
        assert!(ItemId::new_from(V0).is_of_tdbid(V1).not());
    }
}
