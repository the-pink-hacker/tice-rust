use anyhow::anyhow;

pub mod asm;
pub mod bin;
pub mod c;

const FONT_PACK_HEADER: &[u8; 8] = b"FONTPACK";
const MAX_FONTS_LENGTH: usize = 127;
const MAX_GLYPHS_LENGTH: usize = u8::MAX as usize + 1;

/// Clamps the number of fonts to `[1, 127]`.
fn get_fonts_length(length: usize) -> anyhow::Result<u8> {
    match length {
        0 => Err(anyhow!("There must be at least one font in a pack.")),
        1..MAX_FONTS_LENGTH => Ok(length as u8),
        MAX_FONTS_LENGTH => Ok(0),
        _ => Err(anyhow!(
            "There can't be more than {MAX_FONTS_LENGTH} fonts in a pack."
        )),
    }
}

/// Clamps the number of glyphs to `[0, 255]`. If the given length is `256`, it's mapped to `0`. A length
/// of `0` and anything above `256` is an error.
fn get_glyphs_length(length: usize) -> anyhow::Result<u8> {
    match length {
        0 => Err(anyhow!("There must be at least one glyph in a font.")),
        1..MAX_GLYPHS_LENGTH => Ok(length as u8),
        MAX_GLYPHS_LENGTH => Ok(0),
        _ => Err(anyhow!(
            "There can't be more than {MAX_GLYPHS_LENGTH} glyphs in a font."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_glyphs_length_normal() {
        let length = get_glyphs_length(1).unwrap();
        assert_eq!(length, 1);
    }

    #[test]
    fn get_glyphs_length_max() {
        let length = get_glyphs_length(256).unwrap();
        assert_eq!(length, 0);
    }

    #[test]
    #[should_panic]
    fn get_glyphs_length_empty() {
        get_glyphs_length(0).unwrap();
    }

    #[test]
    #[should_panic]
    fn get_glyphs_length_over() {
        get_glyphs_length(257).unwrap();
    }
}
