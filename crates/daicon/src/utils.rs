use std::mem::size_of;

use thiserror::Error;

use crate::{Entry, Header};

/// Utility helper extensions for Header.
pub trait HeaderExt {
    /// Get the absolute offset of the end of the table, so the offset of the first byte after it.
    fn end_of_table(&self, offset: u64) -> u64;
}

impl HeaderExt for Header {
    fn end_of_table(&self, offset: u64) -> u64 {
        offset + size_of::<Header>() as u64 + (size_of::<Entry>() as u64 * self.capacity() as u64)
    }
}

/// Utility helper extensions for Entry.
pub trait EntryExt {
    /// Get the true offset in the file for this entry.
    fn offset(&self, end_of_table: u64) -> u64;

    /// Set the true offset in the file for this entry.
    ///
    /// Returns `Err` if the offset cannot be represented in the table.
    fn set_offset(&mut self, offset: u64, end_of_table: u64) -> Result<(), OffsetOutOfRangeError>;
}

impl EntryExt for Entry {
    fn offset(&self, end_of_table: u64) -> u64 {
        end_of_table + self.relative_offset() as u64
    }

    fn set_offset(&mut self, end_of_table: u64, offset: u64) -> Result<(), OffsetOutOfRangeError> {
        let relative_offset = offset
            .checked_sub(end_of_table)
            .ok_or(OffsetOutOfRangeError)?
            .try_into()
            .map_err(|_| OffsetOutOfRangeError)?;
        self.set_relative_offset(relative_offset);

        Ok(())
    }
}

#[derive(Error, Debug)]
#[error("the given offset is out of range of an entry relative offset")]
pub struct OffsetOutOfRangeError;
