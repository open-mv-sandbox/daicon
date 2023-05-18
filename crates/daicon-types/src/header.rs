use std::num::NonZeroU32;

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
    next: u32,
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
    pub fn capacity(&self) -> u16 {
        self.capacity
    }

    pub fn set_capacity(&mut self, value: u16) {
        self.capacity = value;
    }

    /// Get the amount of entries that contain valid data in this table.
    pub fn valid(&self) -> u16 {
        self.valid
    }

    pub fn set_valid(&mut self, value: u16) {
        self.valid = value;
    }

    /// Get the offset of the next table.
    pub fn next(&self) -> Option<NonZeroU32> {
        NonZeroU32::new(self.next)
    }

    pub fn set_next(&mut self, value: Option<NonZeroU32>) {
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
