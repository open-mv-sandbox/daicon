use std::{
    fs::File,
    io::{Seek, SeekFrom, Write},
};

use anyhow::Error;
use bytemuck::bytes_of;
use daicon::{
    utils::{EntryExt, HeaderExt},
    Entry, Header, SIGNATURE,
};
use uuid::{uuid, Uuid};

const TEXT: &str = include_str!("lipsum.txt");
const METADATA: &str = "Fancy metadata the reader isn't going to use!";

const TEXT_EXAMPLE_ID: Uuid = uuid!("37cb72a4-caab-440c-8b7c-869019ed348e");
const METADATA_EXAMPLE_ID: Uuid = uuid!("c18af4e8-fced-4890-b18c-547fcc7df67b");

fn main() -> Result<(), Error> {
    // Create and write signature
    let mut file = File::create("./lorem.example-text")?;
    file.write_all(SIGNATURE)?;

    // Write the component table, pre-allocating space for 2 entries
    let header_offset = file.stream_position()?;
    let mut header = Header::default();
    header.set_capacity(2);
    header.set_length(2);
    file.write_all(bytes_of(&header))?;

    // Skip forward to the end of the table
    let start_of_table = file.stream_position()?;
    let offset = header.end_of_table(header_offset);
    file.seek(SeekFrom::Start(offset))?;

    // Write the data contents this file should contain
    let text_offset = write_entry_data(&mut file, TEXT.as_bytes())?;
    let meta_offset = write_entry_data(&mut file, METADATA.as_bytes())?;

    // Go back to the first entry in the file, so we can write the entry data
    file.seek(SeekFrom::Start(start_of_table))?;

    // Write the text entry
    let mut entry = Entry::default();
    entry.set_id(TEXT_EXAMPLE_ID);
    entry.set_offset(offset, text_offset)?;
    entry.set_size(TEXT.as_bytes().len() as u32);
    file.write_all(bytes_of(&entry))?;

    // Write an additional metadata entry
    // This isn't going to be read by the reader example, it demonstrates how you can add
    // arbitrary data to a daicon file without conflicting with existing data.
    let mut entry = Entry::default();
    entry.set_id(METADATA_EXAMPLE_ID);
    entry.set_offset(offset, meta_offset)?;
    entry.set_size(METADATA.as_bytes().len() as u32);
    file.write_all(bytes_of(&entry))?;

    Ok(())
}

fn write_entry_data(file: &mut File, data: &[u8]) -> Result<u64, Error> {
    let offset = file.stream_position()?;
    file.write_all(data)?;
    Ok(offset)
}
