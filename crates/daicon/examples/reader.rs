use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{Read, Seek, SeekFrom},
    num::NonZeroU64,
};

use anyhow::{bail, Context, Error};
use bytemuck::{bytes_of_mut, from_bytes, Zeroable};
use daicon::{data::RegionData, ComponentEntry, ComponentTableHeader, SIGNATURE};
use uuid::{uuid, Uuid};

const TEXT_COMPONENT_EXAMPLE_ID: Uuid = uuid!("37cb72a4-caab-440c-8b7c-869019ed348e");

fn main() -> Result<(), Error> {
    let mut file = File::open("./lorem.example-text")?;

    // Validate signature
    let mut data = [0u8; 8];
    file.read_exact(&mut data)?;
    if data != SIGNATURE {
        bail!("invalid signature");
    }

    // Get the component entry that contains the region
    let table = read_components(&mut file)?;
    let entry = table
        .get(&TEXT_COMPONENT_EXAMPLE_ID)
        .context("no text component example in file")?;
    let region = from_bytes::<RegionData>(&entry.data);

    // Read the text data
    let mut data = vec![0u8; region.size() as usize];
    let offset = region.offset(entry.header_offset);
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut data)?;

    // Convert to UTF-8 and print
    let text = std::str::from_utf8(&data)?;
    println!("\n{}", text);

    Ok(())
}

fn read_components(file: &mut File) -> Result<HashMap<Uuid, ReadComponentEntry>, Error> {
    let mut entries = HashMap::new();
    let mut next = NonZeroU64::new(8);

    let mut checked = HashSet::new();

    while let Some(current) = next {
        // Prevent loops
        if checked.contains(&current) {
            println!("table loop in file");
            break;
        }
        checked.insert(current);

        // Read the table's data
        let (header, table_entries) = read_table(file, current.get())?;

        for (uuid, data) in table_entries {
            entries.entry(uuid).or_insert_with(|| ReadComponentEntry {
                header_offset: header.entries_offset(),
                data,
            });
        }

        // Keep following the next table until there's no next
        next = header.next_table_offset();
    }

    Ok(entries)
}

struct ReadComponentEntry {
    header_offset: u64,
    data: [u8; 8],
}

fn read_table(
    file: &mut File,
    offset: u64,
) -> Result<(ComponentTableHeader, Vec<(Uuid, [u8; 8])>), Error> {
    file.seek(SeekFrom::Start(offset))?;

    // Read header
    let mut header = ComponentTableHeader::zeroed();
    file.read_exact(bytes_of_mut(&mut header))?;

    // Read entries
    let mut entries = Vec::new();
    for _ in 0..header.length() {
        let mut entry = ComponentEntry::zeroed();
        file.read_exact(bytes_of_mut(&mut entry))?;

        println!("found component: {}", entry.type_id());

        entries.push((entry.type_id(), entry.data().clone()));
    }

    Ok((header, entries))
}
