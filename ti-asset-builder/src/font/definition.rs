/// Doc comments adapted from [CE-Toolchain](https://ce-programming.github.io/toolchain/libraries/fontlibc.html)
use std::path::PathBuf;

use ascii::AsciiChar;
use serde::Deserialize;

const DEFAULT_CODE_PAGE: &str = "ASCII";

// TODO: Check if there's a better way to wrap TOML structs
/// Wraps the definition so there's no root fields
#[derive(Debug, Clone, Deserialize)]
pub struct FontPackDefinitionWrapper {
    pub pack: FontPackDefinition,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FontPackDefinition {
    pub metadata: FontPackMetadata,
    /// Relative paths, from the font pack definition, to each font definition without the `.toml`
    /// extension.
    pub fonts: Vec<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FontPackMetadata {
    /// A **short**, human-readable typeface name, such as "Times".
    #[serde(default)]
    pub family_name: Option<String>,
    /// A **short** string naming the typeface designer.
    #[serde(default)]
    pub author: Option<String>,
    /// A **short** copyright claim.
    #[serde(default)]
    pub pseudocopyright: Option<String>,
    /// A **brief** description of the font.
    #[serde(default)]
    pub description: Option<String>,
    /// This is a `String`, so while this should be something like `"1.0.0.0"`. It could also be
    /// something like `"1 June 2019"`, or even `"Hahaha versioning is overrated!"`
    #[serde(default)]
    pub version: Option<String>,
    /// Suggested values: “ASCII” “TIOS” “ISO-8859-1” “Windows 1252” “Calculator 1252”.
    #[serde(default = "FontPackMetadata::default_code_page")]
    pub code_page: Option<String>,
}

impl FontPackMetadata {
    fn default_code_page() -> Option<String> {
        Some(DEFAULT_CODE_PAGE.to_string())
    }
}

// TODO: Check if there's a better way to wrap TOML structs
/// Wraps the definition so there's no root fields
#[derive(Debug, Clone, Deserialize)]
pub struct FontDefinitionWrapper {
    pub font: FontDefinition,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct FontDefinition {
    /// Currently, only zero is accepted with fontlibc.
    pub version: u8,
    /// Height in pixels not including space above/below.
    pub height: u8,
    /// Specifies how much to move the cursor left after each glyph.
    /// Total movement is width - overhang.
    pub italic_space_adjust: u8,
    /// Suggests adding blank space above each line of text.
    pub space_above: u8,
    /// Suggests adding blank space below each line of text.
    pub space_below: u8,
    /// Specifies the boldness of the font.
    pub weight: Option<FontWeight>,
    /// Specifies the style of the font.
    #[serde(default)]
    pub style: FontStyle,
    /// For layout, allows aligning text of differing fonts vertically.
    /// This counts pixels going down, i.e. 0 means the top of the glyph.
    pub cap_height: u8,
    /// For layout, allows aligning text of differing fonts vertically.
    /// This counts pixels going down, i.e. 0 means the top of the glyph.
    pub x_height: u8,
    /// For layout, allows aligning text of differing fonts vertically.
    /// This counts pixels going down, i.e. 0 means the top of the glyph.
    pub baseline_height: u8,
    pub glyphs: Vec<FontGlyph>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum FontWeight {
    Thin = 0x20,
    ExtraLight = 0x30,
    Light = 0x40,
    Semilight = 0x60,
    Normal = 0x80,
    Medium = 0x90,
    Semibold = 0xA0,
    Bold = 0xC0,
    ExtraBold = 0xE0,
    Black = 0xF0,
}

impl From<FontWeight> for u8 {
    fn from(value: FontWeight) -> Self {
        value as u8
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(default)]
pub struct FontStyle {
    /// Clear = sans-serif font.
    pub serif: bool,
    /// Oblique is slanted like italic text, but with the cursive-like styling.
    pub oblique: bool,
    /// If both italic and oblique are set, then assume there’s no difference between oblique and
    /// italic styles.
    pub italic: bool,
    /// Monospaced font. This is not enforced; a variable-width font can claim to be monospaced!
    pub monospaced: bool,
}

impl From<FontStyle> for u8 {
    fn from(value: FontStyle) -> Self {
        let mut output = 0;

        if value.serif {
            output |= 0b0000_0001;
        }

        if value.oblique {
            output |= 0b0000_0010;
        }

        if value.italic {
            output |= 0b0000_0100;
        }

        if value.monospaced {
            output |= 0b0000_1000;
        }

        output
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FontGlyph {
    pub index: GlyphIndex,
    /// A path relative from the font definition to the glyph's PNG without the `.png` extension.
    pub source: PathBuf,
}

/// Where a glyph is mapped in the code page.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum GlyphIndex {
    Number(u8),
    Char(AsciiChar),
}

impl From<GlyphIndex> for u8 {
    fn from(value: GlyphIndex) -> Self {
        match value {
            GlyphIndex::Number(value) => value,
            GlyphIndex::Char(value) => value as u8,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_test::{Token, assert_de_tokens, assert_de_tokens_error};

    use super::*;

    #[test]
    fn glyph_index_de_number() {
        assert_de_tokens(&GlyphIndex::Number(12), &[Token::U8(12)]);
    }

    // Strings containing only one ASCII char are good
    #[test]
    fn glyph_index_de_char_str() {
        assert_de_tokens(&GlyphIndex::Char(AsciiChar::a), &[Token::Str("a")]);
    }

    // Confirm non-printable chars work
    #[test]
    fn glyph_index_de_char_str_nonprintable() {
        assert_de_tokens(&GlyphIndex::Char(AsciiChar::LineFeed), &[Token::Str("\n")]);
    }

    // Disallow non-ASCII chars
    #[test]
    fn glyph_index_de_char_str_nonascii() {
        assert_de_tokens_error::<GlyphIndex>(
            &[Token::Str("é")],
            "data did not match any variant of untagged enum GlyphIndex",
        );
    }

    // Chars within the ASCII range are good
    #[test]
    fn glyph_index_de_char_char() {
        assert_de_tokens(&GlyphIndex::Char(AsciiChar::a), &[Token::Char('a')]);
    }

    // Confirm non-printable chars work
    #[test]
    fn glyph_index_de_char_char_nonprintable() {
        assert_de_tokens(&GlyphIndex::Char(AsciiChar::LineFeed), &[Token::Char('\n')]);
    }

    // Disallow non-ASCII chars
    #[test]
    fn glyph_index_de_char_char_nonascii() {
        assert_de_tokens_error::<GlyphIndex>(
            &[Token::Char('é')],
            "data did not match any variant of untagged enum GlyphIndex",
        );
    }

    #[test]
    fn font_weight_de_thin() {
        assert_de_tokens(
            &FontWeight::Thin,
            &[
                Token::Enum { name: "FontWeight" },
                Token::Str("thin"),
                Token::Unit,
            ],
        );
    }

    // Confirm casing is snake_case
    #[test]
    fn font_weight_de_extra_bold() {
        assert_de_tokens(
            &FontWeight::ExtraBold,
            &[
                Token::Enum { name: "FontWeight" },
                Token::Str("extra_bold"),
                Token::Unit,
            ],
        );
    }
}
