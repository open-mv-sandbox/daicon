mod indices;
mod service;
mod table;

pub use self::service::open_file_source;

pub struct FileSourceOptions {
    first_table: Option<u64>,
    allocate_capacity: u16,
}

impl FileSourceOptions {
    /// Set the first table's offset.
    ///
    /// By giving an offset, the file source will start by reading all tables in sequence.
    /// This will happen concurrently with resolving get actions.
    pub fn first_table(mut self, value: u64) -> Self {
        self.first_table = Some(value);
        self
    }

    /// Sets the default capacity of new created tables.
    pub fn allocate_capacity(mut self, value: u16) -> Self {
        self.allocate_capacity = value;
        self
    }
}

impl Default for FileSourceOptions {
    fn default() -> Self {
        Self {
            first_table: None,
            allocate_capacity: 256,
        }
    }
}
