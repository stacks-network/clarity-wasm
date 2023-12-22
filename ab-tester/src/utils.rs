use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{BufWriter, BufReader};
use std::path::{PathBuf, Path};
use std::time::Duration;

use color_eyre::eyre::bail;
use color_eyre::Result;
use log::debug;

pub fn try_convert_i32_to_bool(val: i32) -> Result<bool> {
    match val {
        0 => Ok(false),
        1 => Ok(true),
        _ => bail!("failed to convert i32 to bool, expected 0 or 1"),
    }
}

pub fn append_to_path(p: impl Into<OsString>, s: impl AsRef<OsStr>) -> PathBuf {
    let mut p = p.into();
    p.push(s);
    p.into()
}

pub fn zstd_compress<P: AsRef<Path>>(path: P ) -> Result<()> {
    let target_path = append_to_path(path.as_ref().as_os_str(), ".zstd");

    debug!("opening source file for compression: {}", path.as_ref().display());
    let source_file = File::open(&path)?;
    let source_reader = BufReader::new(source_file);

    debug!("opening target file for compression: {}", target_path.display());
    let mut target_file = File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(target_path)?;
    let target_writer = BufWriter::new(&mut target_file);

    debug!("compressing file");
    zstd::stream::copy_encode(source_reader, target_writer, 5)?;
    debug!("compression finished, flushing file");
    target_file.sync_all()?;

    std::thread::sleep(Duration::from_millis(500));

    Ok(())
}

pub fn zstd_decompress<P: AsRef<Path>>(path: P) -> Result<()> {
    let target_path = path.as_ref().with_extension("");

    debug!("opening source file for decompression: {}", path.as_ref().display());
    let source_file = File::open(&path)?;
    let source_reader = BufReader::new(source_file);

    debug!("opening target file for decompression: {}", target_path.display());
    let mut target_file = File::options()
        .write(true)
        .create(true)
        .truncate(true)
        .open(target_path)?;
    let target_writer = BufWriter::new(&mut target_file);

    debug!("decompressing file");
    zstd::stream::copy_decode(source_reader, target_writer)?;
    debug!("decompression finished, flushing file");
    target_file.sync_all()?;

    std::thread::sleep(Duration::from_millis(500));

    Ok(())
}