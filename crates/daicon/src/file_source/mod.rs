mod indices;
mod service;
mod table;

pub use self::service::open_file_source;

pub struct FileSourceOptions {
    open_table: Option<u64>,
    allocate_capacity: u16,
}

impl FileSourceOptions {
    /// Set the offset of the first table to open.
    ///
    /// If given, before writing any new tables the implementation will read all existing tables.
    /// If not given, a new table will be inserted.
    pub fn open_table(mut self, value: u64) -> Self {
        self.open_table = Some(value);
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
            open_table: None,
            allocate_capacity: 256,
        }
    }
}
