use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};

use crate::SIGNATURE;

/// Header of a daicon table.
///
/// When creating a new table for writing, using the `Default` implementation will automatically
/// fill the signature.
#[derive(Pod, Zeroable, PartialEq, Hash, Debug, Clone, Copy)]
#[repr(C)]
pub struct Header {
    signature: u32,
    capacity: u16,
    valid: u16,
    offset: u64,
    next: u64,
}

impl Header {
    /// Get the magic signature field of this table.
    pub fn signature(&self) -> u32 {
        self.signature
    }

    /// Set `signature`.
    pub fn set_signature(&mut self, value: u32) {
        self.signature = value;
    }

    /// Get the amount of indices of allocated space available in this table.
    pub fn capacity(&self) -> u16 {
        self.capacity
    }

    /// Set `capacity`.
    pub fn set_capacity(&mut self, value: u16) {
        self.capacity = value;
    }

    /// Get the amount of indices that contain valid data in this table.
    pub fn valid(&self) -> u16 {
        self.valid
    }

    /// Set `valid`.
    pub fn set_valid(&mut self, value: u16) {
        self.valid = value;
    }

    /// Get the offset that all indices are relative to.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Set `offset`.
    pub fn set_offset(&mut self, value: u64) {
        self.offset = value;
    }

    /// Get the offset of the next table.
    pub fn next(&self) -> Option<NonZeroU64> {
        NonZeroU64::new(self.next)
    }

    /// Set `next`.
    pub fn set_next(&mut self, value: Option<NonZeroU64>) {
        self.next = value.map(|v| v.get()).unwrap_or(0);
    }

    /// Returns true if this header has a valid signature.
    pub fn is_valid(&self) -> bool {
        self.signature == SIGNATURE
    }
}

impl Default for Header {
    fn default() -> Self {
        Self {
            signature: SIGNATURE,
            ..Self::zeroed()
        }
    }
}
