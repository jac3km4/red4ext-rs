use std::iter::FusedIterator;
use std::{mem, ptr, slice};

use super::{CName, IAllocator};
use crate::raw::root::RED4ext as red;

const INVALID_INDEX: u32 = u32::MAX;

/// Owns an engine hash map.
/// Dropping the map drops its keys and values and releases engine-allocated storage.
/// Growth requires an initialized engine allocator.
///
/// SDK: [HashMap](https://github.com/WopsS/RED4ext.SDK/blob/2216befc23924b46996a9335043aebb31ba6a757/include/RED4ext/HashMap.hpp).
#[derive(Debug)]
#[repr(transparent)]
pub struct RedHashMap<K, V>(red::HashMap<K, V>);

impl<K, V> RedHashMap<K, V> {
    /// Returns the value for an equal key, or `None` if absent.
    #[inline]
    pub fn get(&self, key: &K) -> Option<&V>
    where
        K: Hash + PartialEq,
    {
        let index = self.find_node_index(key, key.hash())?;
        unsafe { Some(&self.live_node(index).value) }
    }

    /// Returns a mutable value for an equal key, or `None` if absent.
    #[inline]
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V>
    where
        K: Hash + PartialEq,
    {
        self.get_by_hash_mut(key, key.hash())
    }

    /// Inserts a key-value pair, returning the previous value if the key exists.
    /// An existing key is retained; a new key may cause engine allocation.
    pub fn insert(&mut self, key: K, value: V) -> Option<V>
    where
        K: Hash + PartialEq,
    {
        let hash = key.hash();

        if self.size() > 0
            && let Some(slot) = self.get_by_hash_mut(&key, hash)
        {
            return Some(mem::replace(slot, value));
        }
        if self.size() + 1 > self.capacity() {
            self.realloc((self.capacity() + self.capacity() / 2).max(4));
        }
        let (node_list, index_table) = self.split_mut();
        Self::push_node(node_list, index_table, hash, key, value);
        self.0.size += 1;

        None
    }

    /// Iterates over borrowed key-value pairs in unspecified order.
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        self.into_iter()
    }

    /// Returns the number of entries.
    #[inline]
    pub fn size(&self) -> u32 {
        self.0.size
    }

    /// Returns the number of entries that fit without growing the map.
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.0.capacity
    }

    fn get_by_hash_mut(&mut self, key: &K, hash: u32) -> Option<&mut V>
    where
        K: PartialEq,
    {
        let index = self.find_node_index(key, hash)?;
        // The bucket chain identifies a live node; the exclusive map borrow
        // keeps its value uniquely borrowed for the returned lifetime.
        unsafe { Some(&mut (*self.0.nodeList.nodes.add(index as usize)).value) }
    }

    fn find_node_index(&self, key: &K, hash: u32) -> Option<u32>
    where
        K: PartialEq,
    {
        let mut cur = *self
            .indexes()
            .get((hash.checked_rem(self.capacity()))? as usize)?;
        while cur != INVALID_INDEX {
            let node = unsafe { self.live_node(cur) };
            if node.hashedKey == hash && node.key == *key {
                return Some(cur);
            }
            cur = node.next;
        }
        None
    }

    fn realloc(&mut self, new_capacity: u32) {
        let new_cap_bytes = new_capacity as usize
            * (mem::size_of::<red::HashMap_Node<K, V>>() + mem::size_of::<u32>());
        let mem = unsafe { self.allocator().alloc_aligned(new_cap_bytes as _, 8) };

        let mut node_list = red::HashMap_NodeList {
            nodes: mem,
            capacity: new_capacity,
            stride: mem::size_of::<red::HashMap_Node<K, V>>() as _,
            ..Default::default()
        };

        let index_table = unsafe {
            mem.byte_add(new_capacity as usize * mem::size_of::<red::HashMap_Node<K, V>>())
        }
        .cast::<u32>();
        let index_table = unsafe { slice::from_raw_parts_mut(index_table, new_capacity as usize) };
        index_table.iter_mut().for_each(|i| *i = INVALID_INDEX);

        if self.capacity() != 0 {
            if self.size() != 0 {
                let (self_nodes, self_indexes) = self.split_mut();
                for idx in self_indexes {
                    let mut cur = *idx;
                    while cur != INVALID_INDEX {
                        let old = unsafe { self_nodes.nodes.add(cur as usize) };
                        unsafe {
                            let next = (*old).next;
                            Self::push_node(
                                &mut node_list,
                                index_table,
                                (*old).hashedKey,
                                ptr::read(&raw const (*old).key),
                                ptr::read(&raw const (*old).value),
                            );
                            cur = next;
                        }
                    }
                    *idx = INVALID_INDEX;
                }
            }
            unsafe { self.allocator().free(self.0.nodeList.nodes) }
        }

        self.0.nodeList = node_list;
        self.0.indexTable = index_table.as_mut_ptr();
        self.0.capacity = new_capacity;
    }

    fn push_node(
        node_list: &mut red::HashMap_NodeList<K, V>,
        index_table: &mut [u32],
        hash: u32,
        key: K,
        value: V,
    ) {
        let node = Self::next_free_node(node_list).unwrap();
        let next = &mut index_table[hash as usize % index_table.len()];
        unsafe {
            (*node).hashedKey = hash;
            ptr::write(&raw mut (*node).key, key);
            ptr::write(&raw mut (*node).value, value);
            (*node).next = *next;
            *next = node.offset_from(node_list.nodes) as _;
        }
    }

    fn next_free_node(
        nl: &mut red::HashMap_NodeList<K, V>,
    ) -> Option<*mut red::HashMap_Node<K, V>> {
        if nl.nextIdx == INVALID_INDEX {
            return None;
        }
        if nl.nextIdx == nl.size {
            let node = unsafe { nl.nodes.add(nl.size as _) };
            if nl.size + 1 < nl.capacity {
                nl.size += 1;
                nl.nextIdx += 1;
            } else {
                nl.nextIdx = INVALID_INDEX;
            }
            return Some(node);
        }
        let node = unsafe { nl.nodes.add(nl.nextIdx as _) };
        nl.nextIdx = unsafe { (*node).next };
        Some(node)
    }

    #[inline]
    fn split_mut(&mut self) -> (&mut red::HashMap_NodeList<K, V>, &mut [u32]) {
        (
            &mut self.0.nodeList,
            if self.0.capacity > 0 {
                unsafe { slice::from_raw_parts_mut(self.0.indexTable, self.0.capacity as _) }
            } else {
                Default::default()
            },
        )
    }

    #[inline]
    fn indexes(&self) -> &[u32] {
        if self.capacity() > 0 {
            unsafe { slice::from_raw_parts(self.0.indexTable, self.0.capacity as _) }
        } else {
            Default::default()
        }
    }

    /// `index` must identify a live node reached through a bucket chain.
    #[inline]
    unsafe fn live_node(&self, index: u32) -> &red::HashMap_Node<K, V> {
        // nodeList.size excludes the last full slot, and unused slots may
        // contain uninitialized keys/values, so no typed node slice is valid.
        unsafe { &*self.0.nodeList.nodes.add(index as usize) }
    }

    #[inline]
    fn allocator(&self) -> &IAllocator {
        unsafe { &*(&self.0.allocator as *const _ as *const IAllocator) }
    }
}

impl<K, V> Drop for RedHashMap<K, V> {
    fn drop(&mut self) {
        if self.capacity() == 0 {
            return;
        }
        for &index in self.indexes() {
            let mut cur = index;
            while cur != INVALID_INDEX {
                unsafe {
                    let node = self.0.nodeList.nodes.add(cur as usize);
                    cur = (*node).next;
                    ptr::drop_in_place(node);
                }
            }
        }
        unsafe { self.allocator().free(self.0.nodeList.nodes) };
    }
}

impl<'a, K, V> IntoIterator for &'a RedHashMap<K, V> {
    type IntoIter = Iter<'a, K, V>;
    type Item = (&'a K, &'a V);

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter {
            current_index: INVALID_INDEX,
            indexes: self.indexes(),
            map: self,
        }
    }
}

/// Borrowed key-value pairs from a [`RedHashMap`], in unspecified order.
#[derive(Debug)]
pub struct Iter<'a, K, V> {
    current_index: u32,
    indexes: &'a [u32],
    map: &'a RedHashMap<K, V>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.current_index != INVALID_INDEX {
                let node = unsafe { self.map.live_node(self.current_index) };
                self.current_index = node.next;
                return Some((&node.key, &node.value));
            }

            let (index, rem) = self.indexes.split_first()?;
            self.current_index = *index;
            self.indexes = rem;
        }
    }
}

impl<K, V> FusedIterator for Iter<'_, K, V> {}

/// Produces the 32-bit hash used by [`RedHashMap`], not Rust's [`std::hash::Hash`].
/// Equal keys must have equal hashes. For engine-owned maps, hashes must also
/// match the SDK's hashing algorithm for the key type.
pub trait Hash {
    /// Returns the key's hash for map lookup and insertion.
    fn hash(&self) -> u32;
}

impl Hash for CName {
    #[inline]
    fn hash(&self) -> u32 {
        let hash = u64::from(*self);
        hash as u32 ^ (hash >> 32) as u32
    }
}

impl Hash for u32 {
    #[inline]
    fn hash(&self) -> u32 {
        *self
    }
}

impl Hash for u64 {
    #[inline]
    fn hash(&self) -> u32 {
        // SDK HashMapHash<uint64_t> hashes all eight bytes in native order.
        self.to_ne_bytes()
            .into_iter()
            .fold(0x811C_9DC5, |hash, byte| {
                (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
            })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::mem::{ManuallyDrop, MaybeUninit};

    use red::Memory::{AllocationResult, IAllocator as RawAllocator, IAllocator__bindgen_vtable};

    use super::*;

    #[test]
    fn u64_hash_matches_sdk_fnv1a32_bytes() {
        // Fixed FNV-1a32 vectors for the eight little-endian bytes used on
        // Windows x64, including zero padding and nonzero upper halves.
        let vectors = [
            (0x0000_0000_0000_0000u64, 0x9BE1_7165),
            (0x0000_0000_0000_0001, 0x3E80_1244),
            (0x0000_0000_FFFF_FFFF, 0x84BF_99C1),
            (0x0000_0001_0000_0000, 0x4BB5_3254),
            (0x0123_4567_89AB_CDEF, 0xE39E_4E75),
            (0xFFFF_FFFF_FFFF_FFFF, 0x6CAE_0A5D),
            // Native bytes are the standard FNV-1a input b"abcdefgh".
            (0x6867_6665_6463_6261, 0x76DA_AA8D),
        ];
        for (key, expected) in vectors {
            assert_eq!(key.hash(), expected, "key {key:#018x}");
        }
    }

    #[test]
    fn u32_hash_remains_identity() {
        for key in [0u32, 1, 0x1234_5678, u32::MAX] {
            assert_eq!(key.hash(), key);
        }
    }

    #[derive(Debug, PartialEq)]
    enum Event {
        Drop(u32),
        Free(usize),
    }

    thread_local! {
        static EVENTS: RefCell<Vec<Event>> = const { RefCell::new(Vec::new()) };
    }

    unsafe extern "C" fn alloc(_: *const RawAllocator, _: u64) -> AllocationResult {
        AllocationResult::default()
    }

    unsafe extern "C" fn alloc_aligned(_: *const RawAllocator, _: u64, _: u32) -> AllocationResult {
        AllocationResult::default()
    }

    unsafe extern "C" fn realloc(
        _: *const RawAllocator,
        _: *mut AllocationResult,
        _: u64,
    ) -> AllocationResult {
        AllocationResult::default()
    }

    unsafe extern "C" fn realloc_aligned(
        _: *const RawAllocator,
        _: *mut AllocationResult,
        _: u64,
        _: u32,
    ) -> AllocationResult {
        AllocationResult::default()
    }

    unsafe extern "C" fn free(_: *const RawAllocator, allocation: *mut AllocationResult) {
        let memory = unsafe { (*allocation).memory };
        EVENTS.with_borrow_mut(|events| events.push(Event::Free(memory as usize)));
    }

    unsafe extern "C" fn sub_28(_: *const RawAllocator, _: *mut std::ffi::c_void) {}

    unsafe extern "C" fn get_handle(_: *const RawAllocator) -> u32 {
        0
    }

    static VTABLE: IAllocator__bindgen_vtable = IAllocator__bindgen_vtable {
        IAllocator_Alloc: alloc,
        IAllocator_AllocAligned: alloc_aligned,
        IAllocator_Realloc: realloc,
        IAllocator_ReallocAligned: realloc_aligned,
        IAllocator_Free: free,
        IAllocator_sub_28: sub_28,
        IAllocator_GetHandle: get_handle,
    };

    // The fake free only records the call. These owners release test storage
    // after the map has destroyed its live entries and called the allocator.
    struct SyntheticMap<K, V> {
        map: RedHashMap<K, V>,
        _nodes: Box<[MaybeUninit<red::HashMap_Node<K, V>>]>,
        _indexes: Box<[u32]>,
    }

    impl<K, V> SyntheticMap<K, V> {
        fn new(capacity: u32) -> Self {
            EVENTS.with_borrow_mut(Vec::clear);
            let mut nodes = Box::<[red::HashMap_Node<K, V>]>::new_uninit_slice(capacity as usize);
            let mut indexes = vec![INVALID_INDEX; capacity as usize].into_boxed_slice();
            let map = RedHashMap(red::HashMap {
                indexTable: indexes.as_mut_ptr(),
                capacity,
                nodeList: red::HashMap_NodeList {
                    nodes: nodes.as_mut_ptr().cast(),
                    capacity,
                    stride: mem::size_of::<red::HashMap_Node<K, V>>() as u32,
                    nextIdx: if capacity == 0 { INVALID_INDEX } else { 0 },
                    ..Default::default()
                },
                allocator: &VTABLE as *const _ as usize,
                ..Default::default()
            });
            Self {
                map,
                _nodes: nodes,
                _indexes: indexes,
            }
        }
    }

    #[derive(PartialEq)]
    struct Tracked(u32);

    impl Hash for Tracked {
        fn hash(&self) -> u32 {
            self.0
        }
    }

    impl Drop for Tracked {
        fn drop(&mut self) {
            EVENTS.with_borrow_mut(|events| events.push(Event::Drop(self.0)));
        }
    }

    #[test]
    fn distinct_keys_with_same_hash_remain_independent() {
        #[derive(PartialEq)]
        struct Key(u32);

        impl Hash for Key {
            fn hash(&self) -> u32 {
                0
            }
        }

        let mut fixture = SyntheticMap::new(4);
        let map = &mut fixture.map;
        assert_eq!(map.insert(Key(1), 10), None);
        assert_eq!(map.insert(Key(2), 20), None);
        assert_eq!(map.size(), 2);
        assert_eq!(map.get(&Key(1)), Some(&10));
        assert_eq!(map.get(&Key(2)), Some(&20));
        assert_eq!(map.get(&Key(3)), None);
        assert_eq!(map.get_mut(&Key(3)), None);
        *map.get_mut(&Key(1)).unwrap() = 11;
        assert_eq!(map.insert(Key(2), 22), Some(20));
        assert_eq!(map.get(&Key(1)), Some(&11));
        assert_eq!(map.get(&Key(2)), Some(&22));
        assert_eq!(map.size(), 2);
    }

    #[test]
    fn u64_keys_preserve_upper_bits() {
        let mut fixture = SyntheticMap::<u64, u32>::new(4);
        let map = &mut fixture.map;
        let low = u64::from(7u32);
        let high = low | (1u64 << 32);
        assert_eq!(map.insert(low, 10), None);
        assert_eq!(map.insert(high, 20), None);
        assert_eq!(map.get(&low), Some(&10));
        assert_eq!(map.get(&high), Some(&20));
        *map.get_mut(&high).unwrap() = 21;
        assert_eq!(map.insert(low, 11), Some(10));
        assert_eq!(map.get(&high), Some(&21));
    }

    #[test]
    fn full_capacity_lookup_and_iteration() {
        let mut fixture = SyntheticMap::<u32, u32>::new(4);
        let map = &mut fixture.map;
        for key in [0, 4, 8, 12] {
            assert_eq!(map.insert(key, key + 1), None);
        }
        assert_eq!(map.0.nodeList.size, 3);
        assert_eq!(map.0.nodeList.nextIdx, INVALID_INDEX);
        for key in [0, 4, 8, 12] {
            assert_eq!(map.get(&key), Some(&(key + 1)));
            *map.get_mut(&key).unwrap() += 1;
        }
        assert_eq!(map.get(&16), None);
        assert_eq!(map.get_mut(&16), None);
        assert_eq!(map.insert(12, 99), Some(14));
        assert_eq!(
            map.iter().collect::<Vec<_>>(),
            vec![(&12, &99), (&8, &10), (&4, &6), (&0, &2)]
        );
        let memory = map.0.nodeList.nodes as usize;
        drop(fixture);
        EVENTS.with_borrow(|events| assert_eq!(*events, vec![Event::Free(memory)]));
    }

    #[test]
    fn full_and_sparse_maps_drop_live_entries_before_storage() {
        for capacity in [2, 4] {
            let mut fixture = SyntheticMap::new(capacity);
            let map = &mut fixture.map;
            for (slot, key) in [(0, 1), (1, 5)] {
                if capacity == 4 {
                    // Initialize only slots 1 and 3, leaving uninitialized holes.
                    map.0.nodeList.size = slot * 2 + 1;
                    map.0.nodeList.nextIdx = slot * 2 + 1;
                }
                map.insert(Tracked(key), Tracked(key * 10));
            }
            let query = ManuallyDrop::new(Tracked(5));
            assert_eq!(map.get(&query).unwrap().0, 50);
            map.get_mut(&query).unwrap().0 = 51;
            let mut iter = map.iter().map(|(k, v)| (k.0, v.0));
            assert_eq!(iter.next(), Some((5, 51)));
            assert_eq!(iter.next(), Some((1, 10)));
            assert!(iter.next().is_none());
            assert!(iter.next().is_none());
            let memory = map.0.nodeList.nodes as usize;
            drop(fixture);
            EVENTS.with_borrow(|events| {
                assert_eq!(
                    *events,
                    vec![
                        Event::Drop(5),
                        Event::Drop(51),
                        Event::Drop(1),
                        Event::Drop(10),
                        Event::Free(memory)
                    ]
                );
            });
        }
    }

    #[test]
    fn empty_maps_drop_storage_only_when_allocated() {
        let fixture = SyntheticMap::<Tracked, Tracked>::new(4);
        let memory = fixture.map.0.nodeList.nodes as usize;
        assert_eq!(fixture.map.iter().count(), 0);
        drop(fixture);
        EVENTS.with_borrow(|events| assert_eq!(*events, vec![Event::Free(memory)]));

        let mut fixture = SyntheticMap::<u32, u32>::new(0);
        // No allocator must be accessed for an unallocated map.
        fixture.map.0.allocator = 0;
        assert_eq!(fixture.map.get(&0), None);
        assert_eq!(fixture.map.get_mut(&0), None);
        assert_eq!(fixture.map.iter().count(), 0);
        drop(fixture);
        EVENTS.with_borrow(|events| assert!(events.is_empty()));
    }
}
