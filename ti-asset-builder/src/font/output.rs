use anyhow::anyhow;

pub mod asm;
pub mod bin;
pub mod c;

const FONT_PACK_HEADER: &[u8; 8] = b"FONTPACK";
const MAX_FONTS_LENGTH: usize = 127;

/// Clamps the number of fonts to `[1, 127]`.
fn get_fonts_length(length: usize) -> anyhow::Result<u8> {
    match length {
        0 => Err(anyhow!("There must be at least one font in a pack.")),
        1..=MAX_FONTS_LENGTH => Ok(length as u8),
        _ => Err(anyhow!(
            "There can't be more than {MAX_FONTS_LENGTH} fonts in a pack."
        )),
    }
}
