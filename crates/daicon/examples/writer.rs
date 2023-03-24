use std::{
    fs::File,
    io::{Seek, SeekFrom, Write},
    mem::size_of,
};

use anyhow::Error;
use bytemuck::bytes_of;
use daicon::{Entry, Header};
use uuid::{uuid, Uuid};

const TEXT: &str = include_str!("lipsum.txt");
const METADATA: &str = "Fancy metadata the reader isn't going to use!";

const TEXT_EXAMPLE_ID: Uuid = uuid!("37cb72a4-caab-440c-8b7c-869019ed348e");
const METADATA_EXAMPLE_ID: Uuid = uuid!("c18af4e8-fced-4890-b18c-547fcc7df67b");

fn main() -> Result<(), Error> {
    // Create the target file
    let mut file = File::create("./lorem.example-text")?;

    // Write the component table, pre-allocating space for 2 entries
    let mut header = Header::default();
    header.set_capacity(2);
    header.set_valid(2);
    file.write_all(bytes_of(&header))?;

    // Skip forward to the end of the table
    let table_start = file.stream_position()?;
    let table_end = table_start + (size_of::<Entry>() as u64 * 2);
    file.seek(SeekFrom::Start(table_end))?;

    // Write the data contents this file should contain
    let text_offset = write_entry_data(&mut file, TEXT.as_bytes())?;
    let meta_offset = write_entry_data(&mut file, METADATA.as_bytes())?;

    // Go back to the first entry in the table, so we can write the entry data
    file.seek(SeekFrom::Start(table_start))?;

    // Write the text entry
    let mut entry = Entry::default();
    entry.set_id(TEXT_EXAMPLE_ID);
    entry.set_offset(text_offset);
    entry.set_size(TEXT.as_bytes().len() as u64);
    file.write_all(bytes_of(&entry))?;

    // Write an additional metadata entry
    // This isn't going to be read by the reader example, it demonstrates how you can add
    // arbitrary data to a daicon file without conflicting with existing data.
    let mut entry = Entry::default();
    entry.set_id(METADATA_EXAMPLE_ID);
    entry.set_offset(meta_offset);
    entry.set_size(METADATA.as_bytes().len() as u64);
    file.write_all(bytes_of(&entry))?;

    Ok(())
}

fn write_entry_data(file: &mut File, data: &[u8]) -> Result<u64, Error> {
    let offset = file.stream_position()?;
    file.write_all(data)?;
    Ok(offset)
}
