use bytemuck::{Pod, Zeroable};
use uuid::Uuid;

/// Entry in a daicon table.
#[derive(Pod, Zeroable, PartialEq, Hash, Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Entry {
    id: [u8; 16],
    offset: u32,
    size: u32,
}

impl Entry {
    /// Get the ID of the entry.
    pub fn id(&self) -> Uuid {
        Uuid::from_bytes_le(self.id)
    }

    pub fn set_id(&mut self, value: Uuid) {
        self.id = value.to_bytes_le();
    }

    /// Get the offset of the entry.
    pub fn offset(&self) -> u32 {
        self.offset
    }

    pub fn set_offset(&mut self, value: u32) {
        self.offset = value;
    }

    /// Get the size of the entry in bytes.
    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn set_size(&mut self, value: u32) {
        self.size = value;
    }
}
