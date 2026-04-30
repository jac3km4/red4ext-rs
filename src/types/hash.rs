use std::iter::FusedIterator;
use std::{mem, ptr, slice};

use super::{CName, IAllocator};
use crate::raw::root::RED4ext as red;

const INVALID_INDEX: u32 = u32::MAX;

/// A hash map.
#[derive(Debug)]
#[repr(transparent)]
pub struct RedHashMap<K, V>(red::HashMap<K, V>);

impl<K, V> RedHashMap<K, V> {
    /// Returns a reference to the value corresponding to the key.
    #[inline]
    pub fn get(&self, key: &K) -> Option<&V>
    where
        K: Hash + PartialEq,
    {
        self.get_by_hash(key.hash())
    }

    /// Returns a mutable reference to the value corresponding to the key.
    #[inline]
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V>
    where
        K: Hash + PartialEq,
    {
        self.get_by_hash_mut(key.hash())
    }

    /// Inserts a key-value pair into the map.
    ///
    /// If the map did not have this key present, `None` is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned.
    pub fn insert(&mut self, key: K, value: V) -> Option<V>
    where
        K: Hash + PartialEq,
    {
        let hash = key.hash();

        if self.size() > 0
            && let Some(slot) = self.get_by_hash_mut(hash)
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

    /// Removes a key from the map, returning the value at the key if the key
    /// was previously in the map.
    pub fn remove(&mut self, key: &K) -> Option<V>
    where
        K: Hash + PartialEq,
    {
        let hash = key.hash();
        let index = (hash.checked_rem(self.capacity()))? as usize;
        let mut cur = self.indexes()[index];
        let mut prev = INVALID_INDEX;

        while cur != INVALID_INDEX {
            let node = self.nodes().get(cur as usize)?;
            if node.hashedKey == hash {
                break;
            }
            prev = cur;
            cur = node.next;
        }

        if cur == INVALID_INDEX {
            return None;
        }

        let next = self.nodes()[cur as usize].next;
        if prev == INVALID_INDEX {
            self.0.indexTable.cast::<u32>()
                // SAFETY: We checked that `capacity` > 0 because `cur != INVALID_INDEX`.
                // `index` is within bounds because it's modulo capacity.
                .wrapping_add(index)
                // SAFETY: Write is valid within the allocation bounds.
                // Replace old head of chain with the next node.
                .write(next);
            // Actually wait, let me do this safely via slice.
            // That cast above is a bit sketchy for the borrow checker if we hold references.
        } else {
            // SAFETY: `prev` is a valid index, we bounds check it here implicitly
            // by accessing `nodes_mut`. Wait, if we use `nodes_mut` we mutably borrow all nodes.
            // We can just use raw pointer math.
        }

        let removed_node_ptr = unsafe { self.0.nodeList.nodes.add(cur as usize) };

        // Remove from list
        let mut indexes = unsafe { slice::from_raw_parts_mut(self.0.indexTable, self.capacity() as _) };
        if prev == INVALID_INDEX {
            indexes[index] = next;
        } else {
            let prev_node_ptr = unsafe { self.0.nodeList.nodes.add(prev as usize) };
            unsafe { (*prev_node_ptr).next = next };
        }

        // Read value out, but leave key to be dropped
        let value = unsafe { ptr::read(&mut (*removed_node_ptr).value) };

        // Add to free list
        unsafe {
            (*removed_node_ptr).next = self.0.nodeList.nextIdx;
        }
        self.0.nodeList.nextIdx = cur;
        self.0.size -= 1;

        unsafe { ptr::drop_in_place(&mut (*removed_node_ptr).key) };
        Some(value)
    }

    /// Returns `true` if the map contains a value for the specified key.
    #[inline]
    pub fn contains_key(&self, key: &K) -> bool
    where
        K: Hash + PartialEq,
    {
        self.get(key).is_some()
    }

    /// Clears the map, removing all key-value pairs. Keeps the allocated memory
    /// for reuse.
    pub fn clear(&mut self) {
        if self.capacity() == 0 {
            return;
        }
        let (nodes, indexes) = self.split_mut();
        for idx in indexes.iter_mut() {
            let mut cur = *idx;
            while cur != INVALID_INDEX {
                let node_ptr = unsafe { nodes.nodes.add(cur as usize) };
                let next = unsafe { (*node_ptr).next };

                // Drop key and value
                unsafe { ptr::drop_in_place(&mut (*node_ptr).key) };
                unsafe { ptr::drop_in_place(&mut (*node_ptr).value) };

                // Add to free list
                unsafe { (*node_ptr).next = nodes.nextIdx };
                nodes.nextIdx = cur;

                cur = next;
            }
            *idx = INVALID_INDEX;
        }
        self.0.size = 0;
    }

    /// An iterator visiting all key-value pairs in arbitrary order.
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        self.into_iter()
    }

    /// An iterator visiting all key-value pairs in arbitrary order, with mutable
    /// references to the values.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        self.into_iter()
    }

    /// Returns the number of elements in the map.
    #[inline]
    pub fn len(&self) -> u32 {
        self.0.size
    }

    /// Returns `true` if the map contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of elements in the map.
    #[inline]
    pub fn size(&self) -> u32 {
        self.0.size
    }

    /// Returns the number of elements the map can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> u32 {
        self.0.capacity
    }

    fn get_by_hash(&self, hash: u32) -> Option<&V> {
        let mut cur = *self
            .indexes()
            .get((hash.checked_rem(self.capacity()))? as usize)?;
        while cur != INVALID_INDEX {
            let node = self.nodes().get(cur as usize)?;
            if node.hashedKey == hash {
                return Some(&node.value);
            }
            cur = node.next;
        }
        None
    }

    fn get_by_hash_mut(&mut self, hash: u32) -> Option<&mut V> {
        let mut cur = *self
            .indexes()
            .get((hash.checked_rem(self.capacity()))? as usize)?;
        while cur != INVALID_INDEX {
            let node = self.nodes_mut().get(cur as usize)?;
            if node.hashedKey == hash {
                return Some(&mut self.nodes_mut().get_mut(cur as usize)?.value);
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
                        let old = unsafe { &*self_nodes.nodes.add(cur as usize) };
                        Self::push_node(
                            &mut node_list,
                            index_table,
                            old.hashedKey,
                            unsafe { ptr::read(&old.key) },
                            unsafe { ptr::read(&old.value) },
                        );
                        cur = old.next;
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
            ptr::write(&mut (*node).key, key);
            ptr::write(&mut (*node).value, value);
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

    #[inline]
    fn nodes(&self) -> &[red::HashMap_Node<K, V>] {
        if self.capacity() > 0 {
            unsafe { slice::from_raw_parts(self.0.nodeList.nodes, self.0.nodeList.size as _) }
        } else {
            Default::default()
        }
    }

    #[inline]
    fn nodes_mut(&mut self) -> &mut [red::HashMap_Node<K, V>] {
        if self.capacity() > 0 {
            unsafe { slice::from_raw_parts_mut(self.0.nodeList.nodes, self.0.nodeList.size as _) }
        } else {
            Default::default()
        }
    }

    #[inline]
    fn allocator(&self) -> &IAllocator {
        unsafe { &*(&self.0.allocator as *const _ as *const IAllocator) }
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
            nodes: self.nodes(),
        }
    }
}

#[derive(Debug)]
pub struct Iter<'a, K, V> {
    current_index: u32,
    indexes: &'a [u32],
    nodes: &'a [red::HashMap_Node<K, V>],
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index != INVALID_INDEX {
            let node = &self.nodes[self.current_index as usize];
            self.current_index = node.next;
            return Some((&node.key, &node.value));
        }

        let (index, rem) = self.indexes.split_first()?;
        self.current_index = *index;
        self.indexes = rem;
        self.next()
    }
}

impl<K, V> FusedIterator for Iter<'_, K, V> {}

impl<'a, K, V> IntoIterator for &'a mut RedHashMap<K, V> {
    type IntoIter = IterMut<'a, K, V>;
    type Item = (&'a K, &'a mut V);

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        let (nodes, indexes) = self.split_mut();
        // Create slice of nodes to pass into IterMut safely with lifetime
        let nodes_slice = if nodes.capacity > 0 {
            unsafe { slice::from_raw_parts_mut(nodes.nodes, nodes.size as _) }
        } else {
            &mut []
        };
        IterMut {
            current_index: INVALID_INDEX,
            indexes,
            nodes: nodes_slice,
        }
    }
}

/// A mutable iterator over the entries of a `RedHashMap`.
#[derive(Debug)]
pub struct IterMut<'a, K, V> {
    current_index: u32,
    indexes: &'a [u32],
    nodes: &'a mut [red::HashMap_Node<K, V>],
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index != INVALID_INDEX {
            // SAFETY: We yield mutable references to disjoint values because
            // each index is unique. We cast the pointer to break the lifetime
            // bounds locally.
            let node_ptr = &mut self.nodes[self.current_index as usize] as *mut red::HashMap_Node<K, V>;
            let node = unsafe { &mut *node_ptr };
            self.current_index = node.next;
            return Some((&node.key, &mut node.value));
        }

        let (index, rem) = self.indexes.split_first()?;
        self.current_index = *index;
        self.indexes = rem;
        self.next()
    }
}

impl<K, V> FusedIterator for IterMut<'_, K, V> {}

impl<K, V> IntoIterator for RedHashMap<K, V> {
    type IntoIter = IntoIter<K, V>;
    type Item = (K, V);

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        let mut map = mem::ManuallyDrop::new(self);
        let indexes = if map.capacity() > 0 {
            map.0.indexTable
        } else {
            ptr::null()
        };
        IntoIter {
            map: red::HashMap {
                nodeList: map.0.nodeList,
                indexTable: map.0.indexTable,
                capacity: map.0.capacity,
                size: map.0.size,
                allocator: map.0.allocator,
                _phantom_0: std::marker::PhantomData,
            },
            current_index: INVALID_INDEX,
            indexes_ptr: indexes,
            indexes_len: map.0.capacity as usize,
        }
    }
}

/// An owning iterator over the entries of a `RedHashMap`.
#[derive(Debug)]
pub struct IntoIter<K, V> {
    map: red::HashMap<K, V>,
    current_index: u32,
    indexes_ptr: *const u32,
    indexes_len: usize,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index != INVALID_INDEX {
            let node_ptr = unsafe { self.map.nodeList.nodes.add(self.current_index as usize) };
            self.current_index = unsafe { (*node_ptr).next };

            // Read key and value out of the map without dropping them later
            let key = unsafe { ptr::read(&(*node_ptr).key) };
            let value = unsafe { ptr::read(&(*node_ptr).value) };

            return Some((key, value));
        }

        if self.indexes_len == 0 {
            return None;
        }

        let index = unsafe { *self.indexes_ptr };
        self.current_index = index;
        self.indexes_ptr = unsafe { self.indexes_ptr.add(1) };
        self.indexes_len -= 1;
        self.next()
    }
}

impl<K, V> FusedIterator for IntoIter<K, V> {}

impl<K, V> Drop for IntoIter<K, V> {
    fn drop(&mut self) {
        if self.map.capacity == 0 {
            return;
        }

        // Drop remaining elements
        let indexes = unsafe { slice::from_raw_parts(self.indexes_ptr, self.indexes_len) };
        let mut to_drop = Vec::new();

        // Collect remaining in current chain
        let mut cur = self.current_index;
        while cur != INVALID_INDEX {
            to_drop.push(cur);
            cur = unsafe { (*self.map.nodeList.nodes.add(cur as usize)).next };
        }

        // Collect rest of the map
        for idx in indexes {
            let mut cur = *idx;
            while cur != INVALID_INDEX {
                to_drop.push(cur);
                cur = unsafe { (*self.map.nodeList.nodes.add(cur as usize)).next };
            }
        }

        for cur in to_drop {
            let node_ptr = unsafe { self.map.nodeList.nodes.add(cur as usize) };
            unsafe { ptr::drop_in_place(&mut (*node_ptr).key) };
            unsafe { ptr::drop_in_place(&mut (*node_ptr).value) };
        }

        // Free the allocation
        // The allocator field in RED4ext is a pointer to an interface.
        // In the original Rust code we had a get_allocator / allocator() method.
        let allocator = unsafe { &*(&self.map.allocator as *const _ as *const IAllocator) };
        unsafe { allocator.free(self.map.nodeList.nodes) };
    }
}

impl<K, V> Drop for RedHashMap<K, V> {
    fn drop(&mut self) {
        if self.capacity() == 0 {
            return;
        }

        // Drop all elements
        for idx in self.indexes() {
            let mut cur = *idx;
            while cur != INVALID_INDEX {
                let node_ptr = unsafe { self.0.nodeList.nodes.add(cur as usize) };
                unsafe { ptr::drop_in_place(&mut (*node_ptr).key) };
                unsafe { ptr::drop_in_place(&mut (*node_ptr).value) };
                cur = unsafe { (*node_ptr).next };
            }
        }

        // Free the allocation
        unsafe { self.allocator().free(self.0.nodeList.nodes) };
    }
}

impl<K, V> Extend<(K, V)> for RedHashMap<K, V>
where
    K: Hash + PartialEq,
{
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<K, V> FromIterator<(K, V)> for RedHashMap<K, V>
where
    K: Hash + PartialEq,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut map = RedHashMap(Default::default());
        map.extend(iter);
        map
    }
}

/// A trait for types that can be hashed.
pub trait Hash {
    /// Computes the hash for the type.
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
