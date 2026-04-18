use std::fmt;
use std::hash::Hash;

use crate::raw::root::RED4ext as red;

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct EntityId(red::ent::EntityID);

impl EntityId {
    #[inline]
    pub const fn is_defined(self) -> bool {
        self.0.hash != 0
    }

    #[inline]
    pub const fn is_static(self) -> bool {
        self.0.hash != 0 && self.0.hash > red::ent::EntityID_DynamicUpperBound
    }

    #[inline]
    pub const fn is_dynamic(self) -> bool {
        self.0.hash != 0 && self.0.hash <= red::ent::EntityID_DynamicUpperBound
    }

    #[inline]
    pub const fn is_persistable(self) -> bool {
        self.0.hash >= red::ent::EntityID_PersistableLowerBound
            && self.0.hash < red::ent::EntityID_PersistableUpperBound
    }

    #[inline]
    pub const fn is_transient(self) -> bool {
        self.0.hash != 0 && !self.is_persistable()
    }
}

impl PartialEq for EntityId {
    fn eq(&self, other: &Self) -> bool {
        self.0.hash == other.0.hash
    }
}

impl Eq for EntityId {}

impl PartialOrd for EntityId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EntityId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.hash.cmp(&other.0.hash)
    }
}

impl Hash for EntityId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash.hash(state);
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self(red::ent::EntityID { hash: 0 })
    }
}

impl From<u64> for EntityId {
    fn from(hash: u64) -> Self {
        Self(red::ent::EntityID { hash })
    }
}

impl From<EntityId> for u64 {
    fn from(EntityId(red::ent::EntityID { hash }): EntityId) -> Self {
        hash
    }
}

impl fmt::Debug for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut attrs = f.debug_set(); // transient and persistable are exclusive
        if self.is_defined() {
            if self.is_dynamic() {
                attrs.entry(&"dynamic");
            } else {
                attrs.entry(&"static");
            }
            if self.is_transient() {
                attrs.entry(&"transient");
            }
        }
        if self.is_persistable() {
            attrs.entry(&"persistable");
        }
        let flags = attrs.finish();
        f.debug_struct("EntityId")
            .field("hash", &self.0.hash)
            .field("flags", &flags)
            .finish_non_exhaustive()
    }
}

/// A simple entity ID representation.
///
/// Flags are displayed as:
/// - `P`ersistable
/// - `D`ynamic
/// - `T`ransient
/// - `S`tatic
impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.is_defined(), self.is_persistable()) {
            (true, persistable) => {
                write!(
                    f,
                    "{} {}",
                    self.0.hash,
                    match (persistable, self.is_dynamic(), self.is_transient()) {
                        (true, true, true) => "(P|D|T)",
                        (true, true, false) => "(P|D)",
                        (true, false, true) => "(P|S|T)",
                        (true, false, false) => "(P|S)",
                        (false, true, true) => "(D|T)",
                        (false, true, false) => "(D)",
                        (false, false, true) => "(S|T)",
                        (false, false, false) => "(S)",
                    }
                )
            }
            (false, false) => write!(f, "undefined"),
            (false, true) => write!(f, "undefined (P)"),
        }
    }
}
