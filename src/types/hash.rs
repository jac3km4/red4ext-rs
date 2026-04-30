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
    /// If the map did not have this key present, [`None`] is returned.
    ///
    /// If the map did have this key present, the value is updated, and the old
    /// value is returned.
    pub fn insert(&mut self, key: K, value: V) -> Option<V>
    where
        K: Hash + PartialEq,
    {
        let hash = key.hash();

        if self.size() > 0 {
            if let Some(slot) = self.get_by_hash_mut(hash) {
                return Some(mem::replace(slot, value));
            }
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
        if self.capacity() == 0 {
            return None;
        }

        let index_pos = (hash.checked_rem(self.capacity()))? as usize;
        let mut cur = *self.indexes().get(index_pos)?;
        let mut prev = INVALID_INDEX;

        while cur != INVALID_INDEX {
            let node = self.nodes().get(cur as usize)?;
            if hash == node.hashedKey {
                let next = node.next;
                if prev == INVALID_INDEX {
                    self.indexes_mut()[index_pos] = next;
                } else {
                    self.nodes_mut()[prev as usize].next = next;
                }

                self.0.size -= 1;
                let old_node = &mut self.nodes_mut()[cur as usize];
                old_node.next = self.0.nodeList.nextIdx;
                self.0.nodeList.nextIdx = cur;

                unsafe {
                    ptr::drop_in_place(&mut old_node.key);
                    return Some(ptr::read(&old_node.value));
                }
            }
            prev = cur;
            cur = node.next;
        }
        None
    }

    /// Clears the map, removing all key-value pairs.
    #[inline]
    pub fn clear(&mut self) {
        if self.capacity() == 0 {
            return;
        }

        let (nodes, indexes) = self.split_mut();
        for idx in indexes {
            let mut cur = *idx;
            while cur != INVALID_INDEX {
                let node = unsafe { &mut *nodes.nodes.add(cur as usize) };
                let next = node.next;
                unsafe {
                    ptr::drop_in_place(&mut node.key);
                    ptr::drop_in_place(&mut node.value);
                }
                node.next = nodes.nextIdx;
                nodes.nextIdx = cur;
                cur = next;
            }
            *idx = INVALID_INDEX;
        }
        self.0.size = 0;
    }

    /// Returns an iterator visiting all key-value pairs in arbitrary order.
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        self.into_iter()
    }

    /// Returns an iterator visiting all key-value pairs in arbitrary order,
    /// with mutable references to the values.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        self.into_iter()
    }

    /// Returns an iterator visiting all keys in arbitrary order.
    #[inline]
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        Iter {
            current_index: INVALID_INDEX,
            indexes: self.indexes(),
            nodes: self.nodes(),
        }
        .map(|(k, _)| k)
    }

    /// Returns an iterator visiting all values in arbitrary order.
    #[inline]
    pub fn values(&self) -> impl Iterator<Item = &V> {
        Iter {
            current_index: INVALID_INDEX,
            indexes: self.indexes(),
            nodes: self.nodes(),
        }
        .map(|(_, v)| v)
    }

    /// Returns the number of elements in the map.
    #[inline]
    pub fn size(&self) -> u32 {
        self.0.size
    }

    /// Returns the number of elements in the map.
    #[inline]
    pub fn len(&self) -> u32 {
        self.0.size
    }

    /// Returns `true` if the map contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.size == 0
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
            &[]
        }
    }

    #[inline]
    fn indexes_mut(&mut self) -> &mut [u32] {
        if self.capacity() > 0 {
            unsafe { slice::from_raw_parts_mut(self.0.indexTable, self.0.capacity as _) }
        } else {
            &mut []
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
            &mut []
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

impl<'a, K, V> IntoIterator for &'a mut RedHashMap<K, V> {
    type IntoIter = IterMut<'a, K, V>;
    type Item = (&'a K, &'a mut V);

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        let (nodes, indexes) = self.split_mut();
        IterMut {
            current_index: INVALID_INDEX,
            indexes,
            nodes: if nodes.capacity > 0 {
                unsafe { slice::from_raw_parts_mut(nodes.nodes, nodes.capacity as _) }
            } else {
                &mut []
            },
        }
    }
}

impl<K, V> IntoIterator for RedHashMap<K, V> {
    type IntoIter = IntoIter<K, V>;
    type Item = (K, V);

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        let mut map = mem::ManuallyDrop::new(self);
        let (nodes, indexes) = map.split_mut();
        IntoIter {
            current_index: INVALID_INDEX,
            map: red::HashMap {
                nodeList: red::HashMap_NodeList {
                    nodes: nodes.nodes,
                    capacity: nodes.capacity,
                    size: nodes.size,
                    nextIdx: nodes.nextIdx,
                    stride: nodes.stride,
                    _phantom_0: std::marker::PhantomData,
                    _phantom_1: std::marker::PhantomData,
                },
                indexTable: if map.capacity() > 0 {
                    map.0.indexTable
                } else {
                    ptr::null_mut()
                },
                capacity: map.capacity(),
                size: map.size(),
                allocator: ptr::null_mut(),
                _phantom_0: std::marker::PhantomData,
                _phantom_1: std::marker::PhantomData,
            },
            indexes: if map.capacity() > 0 {
                unsafe { slice::from_raw_parts_mut(map.0.indexTable, map.capacity() as _) }
            } else {
                &mut []
            },
            nodes: if nodes.capacity > 0 {
                unsafe { slice::from_raw_parts_mut(nodes.nodes, nodes.capacity as _) }
            } else {
                &mut []
            },
            allocator: map.allocator() as *const _ as *mut _,
        }
    }
}

impl<K, V> Drop for RedHashMap<K, V> {
    #[inline]
    fn drop(&mut self) {
        if self.capacity() == 0 {
            return;
        }

        let (nodes, indexes) = self.split_mut();
        for idx in indexes {
            let mut cur = *idx;
            while cur != INVALID_INDEX {
                let node = unsafe { &mut *nodes.nodes.add(cur as usize) };
                let next = node.next;
                unsafe {
                    ptr::drop_in_place(&mut node.key);
                    ptr::drop_in_place(&mut node.value);
                }
                cur = next;
            }
        }

        unsafe {
            self.allocator().free(nodes.nodes);
        }
    }
}

#[derive(Debug)]
pub struct Iter<'a, K, V> {
    current_index: u32,
    indexes: &'a [u32],
    nodes: &'a [red::HashMap_Node<K, V>],
}

impl<'a, K: 'a, V: 'a> Iterator for Iter<'a, K, V> {
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

impl<'a, K: 'a, V: 'a> FusedIterator for Iter<'a, K, V> {}

#[derive(Debug)]
pub struct IterMut<'a, K, V> {
    current_index: u32,
    indexes: &'a [u32],
    nodes: &'a mut [red::HashMap_Node<K, V>],
}

impl<'a, K: 'a, V: 'a> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index != INVALID_INDEX {
            let node = unsafe { &mut *(self.nodes.as_mut_ptr().add(self.current_index as usize)) };
            self.current_index = node.next;
            return Some((&node.key, &mut node.value));
        }

        let (index, rem) = self.indexes.split_first()?;
        self.current_index = *index;
        self.indexes = rem;
        self.next()
    }
}

impl<'a, K: 'a, V: 'a> FusedIterator for IterMut<'a, K, V> {}

#[derive(Debug)]
pub struct IntoIter<K, V> {
    current_index: u32,
    map: red::HashMap<K, V>,
    indexes: *mut [u32],
    nodes: *mut [red::HashMap_Node<K, V>],
    allocator: *mut IAllocator,
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index != INVALID_INDEX {
            let nodes = unsafe { &mut *self.nodes };
            let node = &mut nodes[self.current_index as usize];
            self.current_index = node.next;
            return Some(unsafe { (ptr::read(&node.key), ptr::read(&node.value)) });
        }

        let indexes = unsafe { &mut *self.indexes };
        if let Some((index, rem)) = indexes.split_first_mut() {
            self.current_index = *index;
            self.indexes = rem;
            self.next()
        } else {
            None
        }
    }
}

impl<K, V> FusedIterator for IntoIter<K, V> {}

impl<K, V> Drop for IntoIter<K, V> {
    #[inline]
    fn drop(&mut self) {
        if self.map.capacity == 0 {
            return;
        }

        let indexes = unsafe { &mut *self.indexes };
        let nodes = unsafe { &mut *self.nodes };

        if self.current_index != INVALID_INDEX {
            let mut cur = self.current_index;
            while cur != INVALID_INDEX {
                let node = &mut nodes[cur as usize];
                let next = node.next;
                unsafe {
                    ptr::drop_in_place(&mut node.key);
                    ptr::drop_in_place(&mut node.value);
                }
                cur = next;
            }
        }

        for idx in indexes {
            let mut cur = *idx;
            while cur != INVALID_INDEX {
                let node = &mut nodes[cur as usize];
                let next = node.next;
                unsafe {
                    ptr::drop_in_place(&mut node.key);
                    ptr::drop_in_place(&mut node.value);
                }
                cur = next;
            }
        }

        unsafe {
            (*self.allocator).free(self.map.nodeList.nodes);
        }
    }
}

/// A trait for types that can be hashed.
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

impl Hash for u32 {
    #[inline]
    fn hash(&self) -> u32 {
        *self
    }
}
