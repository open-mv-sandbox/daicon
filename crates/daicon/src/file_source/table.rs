use std::{
    io::{Cursor, Read, Write},
    num::NonZeroU64,
};

use anyhow::Error;
use bytemuck::{bytes_of, bytes_of_mut, cast_slice_mut};
use daicon_types::{Header, Id, Index};
use stewart::Handler;
use uuid::Uuid;

/// Cached in-memory file table.
pub struct Table {
    table_offset: u64,
    dirty: Option<Vec<(Uuid, Handler<Uuid>)>>,

    entries_offset: u64,
    capacity: u16,
    entries: Vec<Index>,
}

impl Table {
    pub fn new(capacity: u16) -> Self {
        Self {
            table_offset: 0,
            dirty: None,

            entries_offset: 0,
            capacity,
            entries: Vec::new(),
        }
    }

    /// Get the offset of the table itself in the file.
    pub fn table_offset(&self) -> u64 {
        self.table_offset
    }

    /// Check if we need to flush, and if so return `Some` with handlers that need to be called on
    /// successful flush.
    pub fn poll_flush(&mut self) -> Option<Vec<(Uuid, Handler<Uuid>)>> {
        self.dirty.take()
    }

    pub fn find(&self, id: Id) -> Option<(u64, u32)> {
        self.entries
            .iter()
            .find(|entry| entry.id() == id)
            .map(|entry| {
                let offset = entry.offset() as u64 + self.entries_offset;
                (offset, entry.size())
            })
    }

    /// Try inserting a new entry, with a handler to report back when flush succeeds.
    pub fn try_insert(
        &mut self,
        id: Id,
        offset: u64,
        size: u32,
        uuid: Uuid,
        on_result: &Handler<Uuid>,
    ) -> bool {
        // Check if we have any room at all
        if self.entries.len() >= self.capacity as usize {
            return false;
        }

        // Check if the offset is in-range
        if offset < self.entries_offset || (offset - self.entries_offset) > u32::MAX as u64 {
            return false;
        }

        // We can now insert it
        let relative = offset - self.entries_offset;

        let mut entry = Index::default();
        entry.set_id(id);
        entry.set_offset(relative as u32);
        entry.set_size(size);

        self.entries.push(entry);

        // Mark dirty since we've now got data to write back
        let dirty = self.dirty.get_or_insert_with(Vec::new);
        dirty.push((uuid, on_result.clone()));

        true
    }

    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        let mut data = Vec::new();

        // Write the header
        let mut header = Header::default();
        header.set_offset(self.entries_offset);
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

    pub fn deserialize(
        table_offset: u64,
        data: &[u8],
    ) -> Result<(Self, Option<NonZeroU64>), Error> {
        let mut data = Cursor::new(data);

        // Read the header
        let mut header = Header::default();
        data.read_exact(bytes_of_mut(&mut header))?;

        // Read entries
        let mut entries = vec![Index::default(); header.valid() as usize];
        data.read_exact(cast_slice_mut(&mut entries))?;

        let table = Self {
            table_offset,
            dirty: None,

            entries_offset: header.offset(),
            capacity: header.capacity(),
            entries,
        };
        Ok((table, header.next()))
    }
}
