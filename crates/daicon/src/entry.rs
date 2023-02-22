use bytemuck::{Pod, TransparentWrapper, Zeroable};
use uuid::Uuid;
use wrapmuck::Wrapmuck;

/// Entry in the component table.
#[derive(TransparentWrapper, Wrapmuck, PartialEq, Hash, Debug, Default, Clone)]
#[repr(transparent)]
pub struct ComponentEntry(ComponentEntryRaw);

impl ComponentEntry {
    pub fn type_id(&self) -> Uuid {
        Uuid::from_bytes_le(self.0.type_id)
    }

    pub fn set_type_id(&mut self, value: Uuid) {
        self.0.type_id = value.to_bytes_le();
    }

    pub fn data(&self) -> &[u8; 8] {
        &self.0.data
    }

    pub fn data_mut(&mut self) -> &mut [u8; 8] {
        &mut self.0.data
    }
}

#[derive(Pod, Zeroable, PartialEq, Hash, Debug, Default, Clone, Copy)]
#[repr(C)]
struct ComponentEntryRaw {
    type_id: [u8; 16],
    data: [u8; 8],
}
