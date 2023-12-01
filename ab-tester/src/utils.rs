use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use color_eyre::eyre::bail;
use color_eyre::Result;

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
