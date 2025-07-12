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
    #[inline]
    pub fn get(&self, key: &K) -> Option<&V>
    where
        K: Hash + PartialEq,
    {
        self.get_by_hash(key.hash())
    }

    #[inline]
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V>
    where
        K: Hash + PartialEq,
    {
        self.get_by_hash_mut(key.hash())
    }

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

    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        self.into_iter()
    }

    #[inline]
    pub fn size(&self) -> u32 {
        self.0.size
    }

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
