use std::{fs::File, io::Write};

use anyhow::Error;
use daicon::{data::RegionData, ComponentEntry, ComponentTableHeader, SIGNATURE};
use uuid::{uuid, Uuid};

const TEXT: &str = include_str!("lipsum.txt");
const TEXT_COMPONENT_EXAMPLE_ID: Uuid = uuid!("37cb72a4-caab-440c-8b7c-869019ed348e");

fn main() -> Result<(), Error> {
    // Pre-calculate the offset where we can start adding data
    let offset =
        (SIGNATURE.len() + ComponentTableHeader::bytes_len() + ComponentEntry::bytes_len()) as u64;

    // Create and write signature
    let mut file = File::create("./lorem.example-text")?;
    file.write_all(SIGNATURE)?;

    // Write the component table
    let mut header = ComponentTableHeader::default();
    header.set_length(1);
    file.write_all(&header)?;

    let mut entry = ComponentEntry::default();
    entry.set_type_id(TEXT_COMPONENT_EXAMPLE_ID);
    let region = RegionData::from_bytes_mut(entry.data_mut());
    region.set_offset(offset, 0)?;
    region.set_size(TEXT.as_bytes().len() as u32);
    file.write_all(&entry)?;

    // Write the text data
    file.write_all(TEXT.as_bytes())?;

    Ok(())
}
