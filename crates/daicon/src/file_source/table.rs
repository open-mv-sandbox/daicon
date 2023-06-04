use std::{
    io::{Cursor, Read, Write},
    num::NonZeroU64,
};

use anyhow::Error;
use bytemuck::{bytes_of, bytes_of_mut, cast_slice_mut};
use daicon_types::{Header, Id, Index};

/// Cached in-memory file table.
pub struct Table {
    offset: u64,
    capacity: u16,
    entries: Vec<Index>,
}

impl Table {
    pub fn new(capacity: u16) -> Self {
        Self {
            offset: 0,
            capacity,
            entries: Vec::new(),
        }
    }

    pub fn find(&self, id: Id) -> Option<(u64, u32)> {
        self.entries
            .iter()
            .find(|entry| entry.id() == id)
            .map(|entry| {
                let offset = entry.offset() as u64 + self.offset;
                (offset, entry.size())
            })
    }

    pub fn try_insert(&mut self, id: Id, offset: u64, size: u32) -> bool {
        // Check if we have any room at all
        if self.entries.len() >= self.capacity as usize {
            return false;
        }

        // Check if the offset is in-range
        if offset < self.offset || (offset - self.offset) > u32::MAX as u64 {
            return false;
        }

        // We can now insert it
        let relative = offset - self.offset;

        let mut entry = Index::default();
        entry.set_id(id);
        entry.set_offset(relative as u32);
        entry.set_size(size);

        self.entries.push(entry);

        true
    }

    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        let mut data = Vec::new();

        // Write the header
        let mut header = Header::default();
        header.set_offset(self.offset);
        header.set_capacity(self.capacity);
        header.set_valid(self.entries.len() as u16);
        data.write_all(bytes_of(&header))?;

        // Write entries
        for entry in &self.entries {
            data.write_all(bytes_of(entry))?;
        }

        // Pad with empty entries
        let empty = Index::default();
        for _ in 0..(self.capacity as usize - self.entries.len()) {
            data.write_all(bytes_of(&empty))?;
        }

        Ok(data)
    }

    pub fn deserialize(data: &[u8]) -> Result<(Self, Option<NonZeroU64>), Error> {
        let mut data = Cursor::new(data);

        // Read the header
        let mut header = Header::default();
        data.read_exact(bytes_of_mut(&mut header))?;

        // Read entries
        let mut entries = vec![Index::default(); header.valid() as usize];
        data.read_exact(cast_slice_mut(&mut entries))?;

        let table = Self {
            offset: header.offset(),
            capacity: header.capacity(),
            entries,
        };
        Ok((table, header.next()))
    }
}
