mod definition;
mod output;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Context;
use log::warn;

use crate::{
    cli::CliFontPackCommand,
    font::definition::{
        FontDefinition, FontDefinitionWrapper, FontGlyph, FontPackDefinition,
        FontPackDefinitionWrapper,
    },
    output::OutputType,
    path::PathExt,
    sprite::{ColorMonochrome, RawImage},
};

#[derive(Debug)]
struct FontGlyphs {
    glyphs: HashMap<u8, (Vec<u8>, u8)>,
    first_glyph: u8,
    last_glyph: u8,
}

impl FontGlyphs {
    async fn new(font: &Path, glyphs: &[FontGlyph]) -> anyhow::Result<Self> {
        let glyph_table = HashMap::with_capacity(glyphs.len());

        let mut output = Self {
            glyphs: glyph_table,
            ..Default::default()
        };

        for glyph in glyphs {
            let path = get_glyph_path(font, &glyph.source)?;
            let (width, _height, pixels) = RawImage::load(&path).await?.into_monochrome();
            let width = width.try_into().with_context(|| {
                format!(
                    "Glyph width must be within range [{}, {}]. Found width: {}",
                    u8::MIN,
                    u8::MAX,
                    width
                )
            })?;
            let bitmap = Self::pixels_to_bytes(width, pixels);
            output.insert(glyph.index.into(), width, bitmap);
        }

        Ok(output)
    }

    fn pixels_to_bytes(width: u8, pixels: Vec<ColorMonochrome>) -> Vec<u8> {
        pixels
            .chunks_exact(width as usize)
            // Process over each row
            .flat_map(|row_pixels| {
                // Convert pairs of 8 into bytes
                row_pixels.chunks(u8::BITS as usize).map(|pixels| {
                    pixels
                        .iter()
                        .enumerate()
                        // Filter empty pixels
                        .flat_map(
                            |(byte_index, &color)| {
                                if color.into() { Some(byte_index) } else { None }
                            },
                        )
                        .fold(0, |byte, byte_index| byte | (1 << (7 - byte_index)))
                })
            })
            .collect()
    }

    fn insert(&mut self, index: u8, width: u8, bitmap: Vec<u8>) {
        self.first_glyph = self.first_glyph.min(index);
        self.last_glyph = self.last_glyph.max(index);
        let old = self.glyphs.insert(index, (bitmap, width));

        if old.is_some() {
            warn!("Glyph is already defined: {index}");
        }
    }

    fn glyph_count(&self) -> u8 {
        // Saturating since a count of 0 is 256
        (self.last_glyph - self.first_glyph).saturating_add(1)
    }
}

impl Default for FontGlyphs {
    fn default() -> Self {
        Self {
            glyphs: HashMap::default(),
            first_glyph: u8::MAX,
            last_glyph: 0,
        }
    }
}

async fn load_pack_definition(path: &Path) -> anyhow::Result<FontPackDefinition> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read font pack definition at {path:?}"))?;
    let definition = toml::from_str::<FontPackDefinitionWrapper>(&raw)
        .with_context(|| format!("Failed to parse font pack definition at {path:?}"))?
        .pack;

    Ok(definition)
}

fn get_font_path(pack: &Path, font: &Path) -> anyhow::Result<PathBuf> {
    pack.relative_parent_suffix(font, ".toml")
}

fn get_glyph_path(font: &Path, glyph: &Path) -> anyhow::Result<PathBuf> {
    font.relative_parent_suffix(glyph, ".png")
}

async fn load_font_definition(path: &Path) -> anyhow::Result<FontDefinition> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read font definition at {path:?}"))?;
    let definition = toml::from_str::<FontDefinitionWrapper>(&raw)
        .with_context(|| format!("Failed to parse font definition at {path:?}"))?
        .font;
    Ok(definition)
}

pub async fn build(command: CliFontPackCommand) -> anyhow::Result<()> {
    let pack_definition_path = command.definition.canonicalize().with_context(|| {
        format!(
            "Failed to get canon font pack definition path: {:?}",
            command.definition
        )
    })?;
    let pack_definition = load_pack_definition(&pack_definition_path).await?;

    let mut fonts = Vec::with_capacity(pack_definition.fonts.len());

    for font_path in &pack_definition.fonts {
        let font_path = get_font_path(&pack_definition_path, font_path)?;
        let font = load_font_definition(&font_path).await?;
        let font_glyphs = FontGlyphs::new(&font_path, &font.glyphs).await?;
        fonts.push((font, font_glyphs));
    }

    match command.output_type {
        OutputType::Assembly => todo!(),
        OutputType::Binary => output::bin::build(&command.output, pack_definition, fonts).await,
        OutputType::C => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_glyphs() {
        let mut font_glyphs = FontGlyphs::default();

        font_glyphs.insert(b'a', 6, vec![1, 2, 3]);
        font_glyphs.insert(b'b', 7, vec![0, 0, 0]);
        font_glyphs.insert(b'd', 8, vec![255, 255, 255]);

        assert_eq!(font_glyphs.first_glyph, b'a');
        assert_eq!(font_glyphs.last_glyph, b'd');
        assert_eq!(font_glyphs.glyph_count(), 4);
        assert_eq!(font_glyphs.glyphs.remove(&b'a'), Some((vec![1, 2, 3], 6)));
        assert_eq!(font_glyphs.glyphs.remove(&b'b'), Some((vec![0, 0, 0], 7)));
        assert_eq!(
            font_glyphs.glyphs.remove(&b'd'),
            Some((vec![255, 255, 255], 8))
        );
        assert!(font_glyphs.glyphs.is_empty());
    }

    #[test]
    fn pixels_to_bytes_6() {
        let bytes = FontGlyphs::pixels_to_bytes(
            6,
            [
                true, false, true, false, true, false, // Row 1
                false, true, false, true, false, true, // Row 2
                false, false, false, true, true, true, // Row 3
            ]
            .into_iter()
            .map(ColorMonochrome::from)
            .collect(),
        );
        let expected = [0b1010_1000, 0b0101_0100, 0b0001_1100];
        assert_eq!(bytes, expected);
    }

    #[test]
    fn pixels_to_bytes_9() {
        let bytes = FontGlyphs::pixels_to_bytes(
            9,
            [
                true, false, true, false, true, false, true, false, true, // Row 1
                false, true, false, true, false, true, false, true, false, // Row 2
                false, false, false, true, true, true, true, true, false, // Row 3
            ]
            .into_iter()
            .map(ColorMonochrome::from)
            .collect(),
        );
        let expected = [
            // Row 1
            0b1010_1010,
            0b1000_0000,
            // Row 2
            0b0101_0101,
            0b0000_0000,
            // Row 3
            0b0001_1111,
            0b0000_0000,
        ];
        assert_eq!(bytes, expected);
    }
}
