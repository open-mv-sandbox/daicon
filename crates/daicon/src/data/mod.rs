//! Common standard "data" types.

use bytemuck::{Pod, Zeroable};
use thiserror::Error;

/// A region containing an offset and size.
#[derive(Pod, Zeroable, Debug, Clone, Copy)]
#[repr(C)]
pub struct RegionData {
    relative_offset: u32,
    size: u32,
}

impl RegionData {
    pub fn relative_offset(&self) -> u32 {
        self.relative_offset
    }

    pub fn set_relative_offset(&mut self, value: u32) {
        self.relative_offset = value;
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn set_size(&mut self, value: u32) {
        self.size = value;
    }

    /// Get the true offset in the file for this entry.
    pub fn offset(&self, entry_offset: u64) -> u64 {
        entry_offset + self.relative_offset() as u64
    }

    /// Set the true offset in the file for this entry.
    pub fn set_offset(
        &mut self,
        offset: u64,
        entry_offset: u64,
    ) -> Result<(), OffsetOutOfRangeError> {
        let relative_offset = offset
            .checked_sub(entry_offset)
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
