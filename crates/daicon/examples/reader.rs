use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use anyhow::{bail, Context, Error};
use bytemuck::{bytes_of_mut, Zeroable};
use daicon::{Entry, Header, SIGNATURE};
use uuid::{uuid, Uuid};

const TEXT_EXAMPLE_ID: Uuid = uuid!("37cb72a4-caab-440c-8b7c-869019ed348e");

fn main() -> Result<(), Error> {
    let mut file = File::open("./lorem.example-text")?;

    // Get the component entry that contains the region
    let table = read_components(&mut file)?;
    let entry = table
        .get(&TEXT_EXAMPLE_ID)
        .context("no text component example in file")?;

    // Read the text data
    let mut data = vec![0u8; entry.size() as usize];
    let offset = entry.offset();
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut data)?;

    // Convert to UTF-8 and print
    let text = std::str::from_utf8(&data)?;
    println!("\n{}", text);

    Ok(())
}

fn read_components(file: &mut File) -> Result<HashMap<Uuid, Entry>, Error> {
    let mut entries = HashMap::new();
    let mut next = Some(0);

    let mut checked = HashSet::new();

    while let Some(current) = next {
        // Prevent loops
        if checked.contains(&current) {
            println!("table loop in file");
            break;
        }
        checked.insert(current);

        // Read the table's data
        let (header, table_entries) = read_table(file, current)?;

        for entry in table_entries {
            entries.insert(entry.id(), entry);
        }

        // Keep following the next table until there's no next
        next = header.next().map(|v| v.get());
    }

    Ok(entries)
}

fn read_table(file: &mut File, offset: u64) -> Result<(Header, Vec<Entry>), Error> {
    file.seek(SeekFrom::Start(offset))?;

    // Read header
    let mut header = Header::zeroed();
    file.read_exact(bytes_of_mut(&mut header))?;

    // Validate header signature
    if header.signature() != SIGNATURE {
        bail!("invalid signature");
    }

    println!("reading header at: {:#010x}", offset);

    // Read entries
    let mut entries = Vec::new();
    for _ in 0..header.valid() {
        let mut entry = Entry::zeroed();
        file.read_exact(bytes_of_mut(&mut entry))?;

        println!("found component: {}", entry.id());

        entries.push(entry);
    }

    Ok((header, entries))
}
