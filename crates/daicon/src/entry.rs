use bytemuck::{Pod, Zeroable};
use uuid::Uuid;

/// Entry in the component table.
#[derive(Pod, Zeroable, PartialEq, Hash, Debug, Default, Clone, Copy)]
#[repr(C, align(8))]
pub struct ComponentEntry {
    type_id: [u8; 16],
    data: [u8; 8],
}

impl ComponentEntry {
    pub fn type_id(&self) -> Uuid {
        Uuid::from_bytes_le(self.type_id)
    }

    pub fn set_type_id(&mut self, value: Uuid) {
        self.type_id = value.to_bytes_le();
    }

    pub fn data(&self) -> &[u8; 8] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [u8; 8] {
        &mut self.data
    }
}
