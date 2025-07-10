use crate::NativeRepr;
use crate::raw::root::RED4ext as red;

#[derive(Debug, Default, Clone, Copy)]
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

    pub const fn from_bytes(name: &[u8]) -> Self {
        const PRIME: u64 = 0x100000001b3;
        const SEED: u64 = 0xCBF29CE484222325;

        let mut hash: u64 = SEED;
        let mut i = 0;

        while i < name.len() {
            let b = name[i];

            if b == b'#' {
                i += 1;
                continue;
            }

            if b == b';' {
                i += 1;
                while i < name.len() && name[i] != b'/' {
                    i += 1;
                }

                if i >= name.len() {
                    break;
                }
            }

            hash ^= b as u64;
            hash = hash.wrapping_mul(PRIME);
            i += 1;
        }

        Self(red::NodeRef {
            hash: if hash == SEED { 0 } else { hash },
        })
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
            u64::from(NodeRef::from_bytes("#mq049_braindance_player".as_bytes())),
            11_169_148_225_646_932_588
        );
        // base\worlds\03_night_city\_compiled\default\exterior_-56_-66_1_0.streamingsector
        // root.nodeRefs[7]
        assert_eq!(
            u64::from(NodeRef::from_bytes(
                "$/03_night_city/c_westbrook/charter_hill/hil_ce_prefabU4INIHQ/ce_wbr_hil_06_prefabOLAKG7I/ce_wbr_hil_06_env_prefabCSSG5EI/#ce_wbr_hil_06_decoration/decoset_pallet_stack_v4_prefabTWZWJMQ/coverObject_004".as_bytes()
            )),
            1_991_599_692_993_048_849
        );
        // base\worlds\03_night_city\_compiled\default\exterior_-56_-66_1_0.streamingsector
        // root.nodeRefs[0]
        assert_eq!(
            u64::from(NodeRef::from_bytes(
                "$/03_night_city/sw1/sw1_env_prefabBCIH5UQ/sw1_architecture_prefabOZJAM6I/exterior_prefabAMKHYOI/arroyo_building_v20_001_prefabM3O3CRQ/decorative_single_door_prefabTIYIITI/{single_door}_prefabPPPD5FQ".as_bytes()
            )),
            3_692_047_525_447_499_933
        );
        // base\worlds\03_night_city\_compiled\default\interior_-9_-47_0_0.streamingsector
        // root.nodeRefs[2]
        assert_eq!(
            u64::from(NodeRef::from_bytes(
                "$/03_night_city/#c_santo_domingo/arroyo/loc_q112_arasaka_industrial_park_warehouse_prefab5EDXBRA/loc_q112_arasaka_industrial_park_warehouse_env_prefab6E43DLA/loc_q112_warehouse_underground_prefabZG2YPFA/loc_q112_warehouse_underground_int_prefab2WZF5RY/loc_q112_warehouse_underground_int_decor_prefabAFW34IY/EntityNode_prefabXTLDHPI".as_bytes()
            )),
            9_372_795_654_866_159_000
        );
    }
}
