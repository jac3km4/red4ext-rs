use std::hash::Hash;

use crate::raw::root::RED4ext as red;
use crate::{NativeRepr, fnv1a64_step};

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct NodeRef(red::NodeRef);

unsafe impl NativeRepr for NodeRef {
    const NAME: &'static str = "worldGlobalNodeRef";
}

impl NodeRef {
    #[inline]
    pub const fn is_defined(self) -> bool {
        self.0.hash != 0
    }

    /// Creates a new `NodeRef` from the given string.
    /// This function just calculates the hash of the string using the FNV-1a algorithm.
    #[inline]
    pub const fn new(name: &str) -> Self {
        Self::from_bytes(name.as_bytes())
    }

    pub const fn from_bytes(mut tail: &[u8]) -> Self {
        const SEED: u64 = 0xCBF2_9CE4_8422_2325;

        let mut hash = SEED;
        while let [byte, t @ ..] = tail {
            tail = t;

            if *byte == b'#' {
                continue;
            }

            if *byte == b';' {
                while let [next, t @ ..] = tail {
                    if *next == b'/' {
                        break;
                    }
                    tail = t;
                }
                continue;
            }

            hash = fnv1a64_step(hash, *byte);
        }

        if hash == SEED {
            hash = 0;
        }

        Self(red::NodeRef { hash })
    }
}

impl PartialEq for NodeRef {
    fn eq(&self, other: &Self) -> bool {
        self.0.hash == other.0.hash
    }
}

impl Eq for NodeRef {}

impl PartialOrd for NodeRef {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NodeRef {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.hash.cmp(&other.0.hash)
    }
}

impl Hash for NodeRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash.hash(state);
    }
}

impl Default for NodeRef {
    fn default() -> Self {
        Self(red::NodeRef { hash: 0 })
    }
}

impl From<u64> for NodeRef {
    fn from(hash: u64) -> Self {
        Self(red::NodeRef { hash })
    }
}

impl From<NodeRef> for u64 {
    fn from(NodeRef(red::NodeRef { hash }): NodeRef) -> Self {
        hash
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn calculate_hashes() {
        // base\quest\minor_quests\mq049\scenes\mq049_braindance.scene
        // root.props[0].findEntityInNodeParams
        assert_eq!(
            u64::from(NodeRef::new("#mq049_braindance_player")),
            11_169_148_225_646_932_588
        );
        // base\worlds\03_night_city\_compiled\default\exterior_-56_-66_1_0.streamingsector
        // root.nodeRefs[7]
        assert_eq!(
            u64::from(NodeRef::new(
                "$/03_night_city/c_westbrook/charter_hill/hil_ce_prefabU4INIHQ/ce_wbr_hil_06_prefabOLAKG7I/ce_wbr_hil_06_env_prefabCSSG5EI/#ce_wbr_hil_06_decoration/decoset_pallet_stack_v4_prefabTWZWJMQ/coverObject_004"
            )),
            1_991_599_692_993_048_849
        );
        // base\worlds\03_night_city\_compiled\default\exterior_-56_-66_1_0.streamingsector
        // root.nodeRefs[0]
        assert_eq!(
            u64::from(NodeRef::new(
                "$/03_night_city/sw1/sw1_env_prefabBCIH5UQ/sw1_architecture_prefabOZJAM6I/exterior_prefabAMKHYOI/arroyo_building_v20_001_prefabM3O3CRQ/decorative_single_door_prefabTIYIITI/{single_door}_prefabPPPD5FQ"
            )),
            3_692_047_525_447_499_933
        );
        // base\worlds\03_night_city\_compiled\default\interior_-9_-47_0_0.streamingsector
        // root.nodeRefs[2]
        assert_eq!(
            u64::from(NodeRef::new(
                "$/03_night_city/#c_santo_domingo/arroyo/loc_q112_arasaka_industrial_park_warehouse_prefab5EDXBRA/loc_q112_arasaka_industrial_park_warehouse_env_prefab6E43DLA/loc_q112_warehouse_underground_prefabZG2YPFA/loc_q112_warehouse_underground_int_prefab2WZF5RY/loc_q112_warehouse_underground_int_decor_prefabAFW34IY/EntityNode_prefabXTLDHPI"
            )),
            9_372_795_654_866_159_000
        );
        // example from Discord
        assert_eq!(
            u64::from(NodeRef::new(
                "$/03_night_city/part_a;#alias_a/part_b;#alias_b"
            )),
            3_039_318_882_151_703_375
        );
        assert_eq!(
            u64::from(NodeRef::new(
                "$/03_night_city/part_a;#alias_a/part_b;#alias_b"
            )),
            u64::from(NodeRef::new("$/03_night_city/part_a/part_b"))
        );
    }
}
