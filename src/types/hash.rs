use super::CName;
use crate::raw::root::RED4ext as red;

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct RedHashMap<K, V>(red::HashMap<K, V>);

impl<K, V> RedHashMap<K, V> {
    const INVALID_INDEX: u32 = u32::MAX;

    pub fn get(&self, key: &K) -> Option<&V>
    where
        K: Hash + PartialEq,
    {
        let hash = key.hash();
        let mut idx = unsafe { self.0.indexTable.add((hash % self.0.capacity) as _).read() };
        while idx != Self::INVALID_INDEX {
            let node = unsafe { &*self.0.nodeList.nodes.add(idx as _) };
            if node.hashedKey == hash && node.key == *key {
                return Some(&node.value);
            }
            idx = node.next;
        }
        None
    }
}

pub trait Hash {
    fn hash(&self) -> u32;
}

impl Hash for CName {
    #[inline]
    fn hash(&self) -> u32 {
        let hash = u64::from(*self);
        hash as u32 ^ (hash >> 32) as u32
    }
}
