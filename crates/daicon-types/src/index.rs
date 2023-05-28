use std::fmt::{self, Debug, Formatter};

use bytemuck::{Pod, Zeroable};

/// Index in a daicon table.
#[derive(Pod, Zeroable, PartialEq, Hash, Default, Clone, Copy)]
#[repr(C)]
pub struct Index {
    id: u32,
    offset: u32,
    size: u32,
}

impl Index {
    /// Get the ID of the entry.
    pub fn id(&self) -> Id {
        Id(self.id)
    }

    pub fn set_id(&mut self, value: Id) {
        self.id = value.0;
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

impl Debug for Index {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Entry({:#010x}) {{ offset: {:#010x}, size: {} }}",
            self.id, self.offset, self.size
        )
    }
}

/// Daicon entry identifier.
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct Id(pub u32);

impl Debug for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Id({:#010x})", self.0)
    }
}
