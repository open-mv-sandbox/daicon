use std::{
    io::{Cursor, Read, Write},
    num::NonZeroU64,
};

use anyhow::Error;
use bytemuck::{bytes_of, bytes_of_mut, cast_slice_mut};
use daicon_types::{Header, Id, Index};

/// Cached in-memory table.
pub struct Table {
    location: u64,
    offset: u64,
    capacity: u16,
    entries: Vec<Index>,
}

impl Table {
    pub fn new(capacity: u16) -> Self {
        Self {
            location: 0,
            offset: 0,
            capacity,
            entries: Vec::new(),
        }
    }

    pub fn location(&self) -> u64 {
        self.location
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

    pub fn can_insert(&self, offset: u64) -> bool {
        // Check if we have any room at all
        if self.entries.len() >= self.capacity as usize {
            return false;
        }

        // Check if the offset is in-range
        if offset < self.offset || (offset - self.offset) > u32::MAX as u64 {
            return false;
        }

        true
    }

    pub fn insert(&mut self, id: Id, offset: u64, size: u32) {
        let relative = offset - self.offset;

        let mut entry = Index::default();
        entry.set_id(id);
        entry.set_offset(relative as u32);
        entry.set_size(size);

        self.entries.push(entry);
    }
}

pub fn serialize_table(table: &Table) -> Result<Vec<u8>, Error> {
    let mut data = Vec::new();

    // Write the header
    let mut header = Header::default();
    header.set_offset(table.offset);
    header.set_capacity(table.capacity);
    header.set_valid(table.entries.len() as u16);
    data.write_all(bytes_of(&header))?;

    // Write entries
    for entry in &table.entries {
        data.write_all(bytes_of(entry))?;
    }

    // Pad with empty entries
    let empty = Index::default();
    for _ in 0..(table.capacity as usize - table.entries.len()) {
        data.write_all(bytes_of(&empty))?;
    }

    Ok(data)
}

pub fn deserialize_table(data: Vec<u8>) -> Result<(Table, Option<NonZeroU64>), Error> {
    let mut data = Cursor::new(data);

    // Read the header
    let mut header = Header::default();
    data.read_exact(bytes_of_mut(&mut header))?;

    // Read entries
    let mut entries = vec![Index::default(); header.valid() as usize];
    data.read_exact(cast_slice_mut(&mut entries))?;

    let table = Table {
        location: 0,
        offset: header.offset(),
        capacity: header.capacity(),
        entries,
    };
    Ok((table, header.next()))
}
