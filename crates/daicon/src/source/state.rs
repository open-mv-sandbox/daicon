use std::{
    io::{Cursor, Read},
    mem::size_of,
};

use anyhow::{bail, Error};
use bytemuck::{bytes_of_mut, cast_slice_mut};
use daicon_types::{Entry, Header};

pub struct SourceState {
    table: Option<TableState>,
}

impl SourceState {
    pub fn new() -> Self {
        Self { table: None }
    }

    // TODO: Refactor into utilities
    pub fn table_mut(&mut self) -> Option<&mut TableState> {
        self.table.as_mut()
    }

    // TODO: Refactor into utilities
    pub fn set_table(&mut self, table: TableState) {
        self.table = Some(table);
    }

    pub fn find(&self, id: u32) -> Option<Entry> {
        self.table.as_ref().and_then(|t| t.find(id))
    }
}

pub struct TableState {
    offset: u64,
    capacity: u16,
    entries: Vec<Entry>,
}

impl TableState {
    pub fn empty(offset: u64, capacity: u16) -> Self {
        Self {
            offset,
            capacity,
            entries: Vec::new(),
        }
    }

    pub fn read(offset: u64, data: Vec<u8>) -> Result<Self, Error> {
        let mut data = Cursor::new(data);

        // Read the header
        let mut header = Header::default();
        data.read_exact(bytes_of_mut(&mut header))?;

        // TODO: Retry if the table's valid data is larger than what we've read, this can happen
        // sometimes

        // Read entries
        let mut entries = vec![Entry::default(); header.valid() as usize];
        data.read_exact(cast_slice_mut(&mut entries))?;

        let table = Self {
            offset,
            capacity: header.capacity(),
            entries,
        };
        Ok(table)
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    fn find(&self, id: u32) -> Option<Entry> {
        self.entries.iter().find(|e| e.id() == id).cloned()
    }

    pub fn try_push(&mut self, id: u32, offset: u64, size: u32) -> Result<(usize, Entry), Error> {
        // Check if we have space for more entries
        if self.entries.len() >= self.capacity as usize {
            bail!("no table capacity left");
        }

        // Check if this entry falls within our supported region
        let relative = offset as i64 - self.offset as i64;
        if relative < 0 || relative > u32::MAX as i64 {
            bail!("offset out of table range");
        }

        // We can take this entry, push it
        let mut entry = Entry::default();
        entry.set_id(id);
        entry.set_offset(relative as u32);
        entry.set_size(size);

        self.entries.push(entry);
        let index = self.entries.len();
        Ok((index, entry))
    }

    pub fn entry_offset(&self, index: usize) -> u64 {
        self.offset + size_of::<Header>() as u64 + (size_of::<Entry>() as u64 * index as u64)
    }

    pub fn write_header(&self, header: &mut Header) {
        header.set_capacity(self.capacity);
        header.set_valid(self.entries.len() as u16);
    }
}
