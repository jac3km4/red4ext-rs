use std::fmt;

use super::TweakDbId;
use crate::raw::root::RED4ext as red;

const DEFAULT_ITEM_ID_RNG_SEED: u32 = 2;

#[derive(Default, Clone, Copy)]
#[repr(transparent)]
/// Item identifier, including record and instance identity.
///
/// SDK: [ItemID](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/NativeTypes.hpp).
pub struct ItemId(red::ItemID);

impl ItemId {
    /// Creates a blueprint-stackable ID for a record, with seed 2 and no flags
    /// or unique counter. Does not create an inventory item or validate the record.
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

    /// Returns the item structure; unknown byte values map to `Invalid`.
    pub fn structure(&self) -> GamedataItemStructure {
        self.0.structure.into()
    }

    /// Returns the item flag; unknown byte values map to `None`.
    pub fn flags(&self) -> GameEItemIdFlag {
        self.0.flags.into()
    }

    /// Returns the item's TweakDB record identifier, including its database offset.
    #[inline]
    pub fn tdbid(&self) -> TweakDbId {
        TweakDbId::from(unsafe { self.0.tdbid.__bindgen_anon_1.value })
    }

    /// Compares record hash and name length, ignoring database offsets.
    #[inline]
    pub const fn is_of_tdbid(&self, tdbid: TweakDbId) -> bool {
        unsafe { self.0.tdbid.__bindgen_anon_1.name }.hash == tdbid.hash()
            && unsafe { self.0.tdbid.__bindgen_anon_1.name }.length == tdbid.len()
    }

    /// Returns whether the record ID is valid and the RNG seed is nonzero.
    /// Does not check whether the record or an inventory item exists.
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

/// Item structure classification. Unknown byte values convert to `Invalid`.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq)]
#[repr(u8)]
pub enum GamedataItemStructure {
    /// Blueprint-stackable structure, used by [`ItemId::new_from`].
    #[default]
    BlueprintStackable = 0,
    /// Stackable structure.
    Stackable = 1,
    /// Unique-item structure.
    Unique = 2,
    /// SDK sentinel for the number of valid structures.
    Count = 3,
    /// Invalid or unrecognized structure.
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

/// Item flags.
/// `None` represents zero; unknown byte values also convert to `None`.
///
/// SDK: [game::EItemIDFlag](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/Scripting/Natives/Generated/game/EItemIDFlag.hpp).
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq)]
#[repr(u8)]
pub enum GameEItemIdFlag {
    /// No recognized flag.
    #[default]
    None = 0,
    /// Preview item flag.
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
