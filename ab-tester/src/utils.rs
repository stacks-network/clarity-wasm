use color_eyre::{Result, eyre::bail};

pub fn try_convert_i32_to_bool(val: i32) -> Result<bool> {
    match val {
        0 => Ok(false),
        1 => Ok(true),
        _ => bail!("failed to convert i32 to bool, expected 0 or 1")
    }
}