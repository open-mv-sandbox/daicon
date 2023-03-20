use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};

/// Header of a daicon table.
#[derive(Pod, Zeroable, PartialEq, Hash, Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Header {
    next_valid: u64,
    capacity: u8,
    next_capacity: u8,
    length: u8,
    // Rerseved values padded to 2 * 8 bytes, we can decide what to do with this later.
    _reserved0: u8,
    _reserved1: u32,
}

impl Header {
    /// Get the amount of entries of allocated space available in this table.
    pub fn capacity(&self) -> u8 {
        self.capacity
    }

    pub fn set_capacity(&mut self, value: u8) {
        self.capacity = value;
    }

    /// Get the amount of entries that contain valid data in this table.
    pub fn length(&self) -> u8 {
        self.length
    }

    pub fn set_length(&mut self, value: u8) {
        self.length = value;
    }

    /// Get the absolute offset of the next table.
    pub fn next_offset(&self) -> Option<NonZeroU64> {
        NonZeroU64::new(self.next_valid)
    }

    pub fn set_next_offset(&mut self, value: Option<NonZeroU64>) {
        self.next_valid = value.map(|v| v.get()).unwrap_or(0);
    }

    /// Get the expected `capacity` value of the next table, for efficient pre-fetching.
    pub fn next_capacity(&self) -> u8 {
        self.next_capacity
    }

    pub fn set_next_capacity_(&mut self, value: u8) {
        self.next_capacity = value;
    }
}
