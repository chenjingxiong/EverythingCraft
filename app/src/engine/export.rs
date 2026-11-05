use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use super::types::SearchResultEntry;

pub const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

pub fn export_to_csv(entries: &[SearchResultEntry], path: &Path) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(UTF8_BOM)?;

    writeln!(writer, "Name,Path,Size,Date Modified,Extension")?;
    for entry in entries {
        writeln!(
            writer,
            "\"{}\",\"{}\",{},{},\"{}\"",
            entry.name.replace('"', "\"\""),
            entry.path.replace('"', "\"\""),
            entry.size,
            entry.modified,
            entry.extension
        )?;
    }
    writer.flush()?;
    Ok(())
}

pub fn export_to_tsv(entries: &[SearchResultEntry], path: &Path) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(UTF8_BOM)?;

    writeln!(writer, "Name\tPath\tSize\tDate Modified\tExtension")?;
    for entry in entries {
        writeln!(
            writer,
            "{}\t{}\t{}\t{}\t{}",
            entry.name, entry.path, entry.size, entry.modified, entry.extension
        )?;
    }
    writer.flush()?;
    Ok(())
}

pub fn export_to_efu(entries: &[SearchResultEntry], path: &Path) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(UTF8_BOM)?;

    // EFU Header: Filename,Size,Date Modified,Date Created,Attributes
    writeln!(writer, "Filename,Size,Date Modified,Date Created,Attributes")?;
    for entry in entries {
        // Windows FILETIME format: (seconds + 11644473600) * 10_000_000
        let win_mod = (entry.modified as u64 + 11644473600) * 10_000_000;
        let win_cre = (entry.created as u64 + 11644473600) * 10_000_000;
        writeln!(
            writer,
            "\"{}\",{},{},{},{}",
            entry.full_path.replace('"', "\"\""),
            entry.size,
            win_mod,
            win_cre,
            entry.attributes
        )?;
    }
    writer.flush()?;
    Ok(())
}

pub fn export_to_txt(entries: &[SearchResultEntry], path: &Path) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(UTF8_BOM)?;

    for entry in entries {
        writeln!(writer, "{}", entry.full_path)?;
    }
    writer.flush()?;
    Ok(())
}

pub fn export_to_json(entries: &[SearchResultEntry], path: &Path) -> std::io::Result<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, entries)?;
    Ok(())
}
