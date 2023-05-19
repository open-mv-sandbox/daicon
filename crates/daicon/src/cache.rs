use std::{
    io::{Cursor, Read},
    mem::size_of,
};

use anyhow::Error;
use bytemuck::{bytes_of_mut, cast_slice_mut, Zeroable};
use daicon_types::{Entry, Header};

/// In-memory cached representation of a table.
pub struct CachedTable {
    offset: u32,
    entries: Vec<Entry>,
    entries_meta: Vec<EntryMeta>,
}

#[derive(Default, Clone)]
pub struct EntryMeta {
    valid: bool,
    allocated: bool,
}

impl CachedTable {
    pub fn new(offset: u32, capacity: usize) -> Self {
        Self {
            offset,
            entries: vec![Entry::zeroed(); capacity],
            entries_meta: vec![EntryMeta::default(); capacity],
        }
    }

    pub fn read(offset: u32, data: Vec<u8>) -> Result<Self, Error> {
        let mut data = Cursor::new(data);

        // Read the header
        let mut header = Header::default();
        data.read_exact(bytes_of_mut(&mut header))?;

        // TODO: Retry if the table's valid data is larger than what we've read, this can happen
        // sometimes

        // Read entries
        let mut entries = vec![Entry::default(); header.capacity() as usize];
        data.read_exact(cast_slice_mut(&mut entries))?;

        // Mark all valid entries as both valid and allocated
        let mut entries_meta = vec![EntryMeta::default(); entries.len()];
        for i in 0..header.valid() as usize {
            entries_meta[i].valid = true;
            entries_meta[i].allocated = true;
        }

        let table = Self {
            offset,
            entries,
            entries_meta,
        };
        Ok(table)
    }

    pub fn find(&self, id: u64) -> Option<Entry> {
        self.entries.iter().find(|e| e.id() == id).cloned()
    }

    pub fn try_allocate(&mut self) -> Option<usize> {
        // Get a slot that hasn't been allocated yet
        let (index, meta) = self
            .entries_meta
            .iter_mut()
            .enumerate()
            .find(|(_, v)| !v.allocated)?;

        // Mark it as allocated
        meta.allocated = true;

        Some(index)
    }

    pub fn entry_offset(&self, index: usize) -> u32 {
        self.offset + size_of::<Header>() as u32 + (size_of::<Entry>() as u32 * index as u32)
    }

    /// Mark that an entry is now available, with the given data.
    ///
    /// Eventually, this will result in the header's count being updated.
    pub fn mark_valid(&mut self, index: usize, entry: Entry) {
        self.entries[index] = entry;
        self.entries_meta[index].valid = true;
    }

    pub fn create_header(&self) -> (Header, u32) {
        // Count the amount of entries until we hit one that isn't valid
        let mut valid = 0;
        for meta in &self.entries_meta {
            if !meta.valid {
                break;
            }

            valid += 1;
        }

        let mut header = Header::default();
        header.set_capacity(self.entries.len() as u16);
        header.set_valid(valid);

        (header, self.offset)
    }
}
