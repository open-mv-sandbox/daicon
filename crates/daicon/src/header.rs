use std::num::NonZeroU64;

use bytemuck::{Pod, TransparentWrapper, Zeroable};
use wrapmuck::Wrapmuck;

/// Header of the component table.
#[derive(TransparentWrapper, Wrapmuck, PartialEq, Hash, Debug, Default, Clone)]
#[repr(transparent)]
pub struct ComponentTableHeader(ComponentTableHeaderRaw);

impl ComponentTableHeader {
    pub fn next_table_offset(&self) -> Option<NonZeroU64> {
        NonZeroU64::new(self.0.next_table_offset)
    }

    pub fn set_next_table_offset(&mut self, value: Option<NonZeroU64>) {
        self.0.next_table_offset = value.map(|v| v.get()).unwrap_or(0);
    }

    pub fn next_table_length_hint(&self) -> u32 {
        self.0.next_table_length_hint
    }

    pub fn set_next_table_length_hint(&mut self, value: u32) {
        self.0.next_table_length_hint = value;
    }

    pub fn length(&self) -> u32 {
        self.0.length
    }

    pub fn set_length(&mut self, value: u32) {
        self.0.length = value;
    }

    pub fn entries_offset(&self) -> u64 {
        self.0.entries_offset
    }

    pub fn set_entries_offset(&mut self, value: u64) {
        self.0.entries_offset = value;
    }
}

#[derive(Pod, Zeroable, PartialEq, Hash, Debug, Default, Clone, Copy)]
#[repr(C)]
struct ComponentTableHeaderRaw {
    next_table_offset: u64,
    next_table_length_hint: u32,
    length: u32,
    entries_offset: u64,
}
