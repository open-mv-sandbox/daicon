use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};

use crate::SIGNATURE;

/// Header of a daicon table.
#[derive(Pod, Zeroable, PartialEq, Hash, Debug, Clone, Copy)]
#[repr(C)]
pub struct Header {
    signature: u32,
    capacity: u8,
    next_capacity: u8,
    length: u8,
    _reserved: u8,
    next: u64,
}

impl Header {
    /// Get the magic signature field of this table.
    pub fn signature(&self) -> u32 {
        self.signature
    }

    pub fn set_signature(&mut self, value: u32) {
        self.signature = value;
    }

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

    /// Get the offset of the next table.
    pub fn next(&self) -> Option<NonZeroU64> {
        NonZeroU64::new(self.next)
    }

    pub fn set_next(&mut self, value: Option<NonZeroU64>) {
        self.next = value.map(|v| v.get()).unwrap_or(0);
    }

    /// Get the expected `capacity` value of the next table, for efficient pre-fetching.
    pub fn next_capacity(&self) -> u8 {
        self.next_capacity
    }

    pub fn set_next_capacity(&mut self, value: u8) {
        self.next_capacity = value;
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
