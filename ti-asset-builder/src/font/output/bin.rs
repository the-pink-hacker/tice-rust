use std::path::Path;

use anyhow::Context;
use log::debug;
use serseg::prelude::*;

use crate::font::{
    FontGlyphs,
    definition::{FontDefinition, FontPackDefinition},
    output::FONT_PACK_HEADER,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SectorId {
    Header,
    Metadata,
    MetadataEnd,
    MetadataStrings,
    FontHeader(usize),
    FontGlyphWidths(usize),
    FontGlyphBitmaps(usize),
    FontGlyphBitmap(usize, u8),
}

type SectorBuilder = SerialSectorBuilder<SectorId>;
type Builder = SerialBuilder<SectorId>;

fn add_font_sectors(
    mut builder: Builder,
    font: FontDefinition,
    font_index: usize,
    mut font_glyphs: FontGlyphs,
) -> anyhow::Result<Builder> {
    let mut widths_builder = SectorBuilder::default();
    let mut bitmap_table_builder = SectorBuilder::default();
    let first_glyph = font_glyphs.first_glyph;
    let glyph_count = font_glyphs.glyph_count();
    let mut glyph_bitmaps = Vec::with_capacity(font_glyphs.glyphs.len());

    for glyph_index in first_glyph..=font_glyphs.last_glyph {
        if let Some((glyph_bitmap, glyph_width)) = font_glyphs.glyphs.remove(&glyph_index) {
            widths_builder = widths_builder.u8(glyph_width);
            bitmap_table_builder = bitmap_table_builder.dynamic_u16(
                SectorId::FontHeader(font_index),
                SectorId::FontGlyphBitmap(font_index, glyph_index),
                0,
            );
            glyph_bitmaps.push((glyph_bitmap, glyph_index));
        } else {
            debug!("Glyph {glyph_index} of font {font_index} is unset and will be defaulted.");
            widths_builder = widths_builder.u8(0);
            // TODO: Add default glyphs
            bitmap_table_builder = bitmap_table_builder.null_16();
        }
    }

    builder = builder
        .sector(
            SectorId::FontHeader(font_index),
            SectorBuilder::default()
                .u8(font.version)
                .u8(font.height)
                .u8(glyph_count)
                .u8(first_glyph)
                .dynamic_u16(
                    SectorId::FontHeader(font_index),
                    SectorId::FontGlyphWidths(font_index),
                    0,
                )
                .dynamic_u16(
                    SectorId::FontHeader(font_index),
                    SectorId::FontGlyphBitmaps(font_index),
                    0,
                )
                .u8(font.italic_space_adjust)
                .u8(font.space_above)
                .u8(font.space_below)
                .u8(font.weight.map(u8::from).unwrap_or_default())
                .u8(font.style)
                .u8(font.cap_height)
                .u8(font.x_height)
                .u8(font.baseline_height),
        )
        .sector(SectorId::FontGlyphWidths(font_index), widths_builder)
        .sector(SectorId::FontGlyphBitmaps(font_index), bitmap_table_builder);

    for (glyph_bitmap, glyph_index) in glyph_bitmaps {
        builder = builder.sector(
            SectorId::FontGlyphBitmap(font_index, glyph_index),
            SectorBuilder::default().bytes(glyph_bitmap),
        );
    }

    Ok(builder)
}

fn generate_serial_builder(
    pack: FontPackDefinition,
    fonts: Vec<(FontDefinition, FontGlyphs)>,
) -> anyhow::Result<Builder> {
    let fonts_length = super::get_fonts_length(fonts.len())?;

    // Pack header
    let mut header_builder = SectorBuilder::default()
        .bytes(*FONT_PACK_HEADER)
        .dynamic_u24(SectorId::Header, SectorId::Metadata, 0)
        .u8(fonts_length);

    // Points to all the fonts in the pack
    for (i, _) in fonts.iter().enumerate() {
        header_builder = header_builder.dynamic_u24(SectorId::Header, SectorId::FontHeader(i), 0);
    }

    // Pack metadata
    let mut metadata_builder =
        SectorBuilder::default().dynamic_u24(SectorId::Metadata, SectorId::MetadataEnd, 0);

    let mut metadata_string_builder = SectorBuilder::default();

    let mut string_index = 0;

    let metadata = pack.metadata;
    let strings = [
        metadata.family_name,
        metadata.author,
        metadata.pseudocopyright,
        metadata.description,
        metadata.version,
        metadata.code_page,
    ];

    // Add each optional string's pointer and data. If the string is `None`, null will be written.
    for text in strings {
        if text.is_empty() {
            metadata_builder = metadata_builder.null_24();
        } else {
            metadata_builder = metadata_builder.dynamic_u24(
                SectorId::Header,
                SectorId::MetadataStrings,
                string_index,
            );
            metadata_string_builder = metadata_string_builder.string(text);
            string_index += 1;
        }
    }

    let mut builder = Builder::default()
        .sector(SectorId::Header, header_builder)
        .sector(SectorId::Metadata, metadata_builder)
        .sector_default(SectorId::MetadataEnd)
        .sector(SectorId::MetadataStrings, metadata_string_builder);

    // Add each font
    for (font_index, (font, font_glyphs)) in fonts.into_iter().enumerate() {
        builder = add_font_sectors(builder, font, font_index, font_glyphs)?;
    }

    debug!("{builder:?}");

    Ok(builder)
}

pub async fn build(
    output: &Path,
    pack: FontPackDefinition,
    fonts: Vec<(FontDefinition, FontGlyphs)>,
) -> anyhow::Result<()> {
    let file = tokio::fs::File::create(output)
        .await
        .with_context(|| format!("Failed to open output font file: {output:?}"))?;
    let mut buffer = tokio::io::BufWriter::new(file);
    generate_serial_builder(pack, fonts)?
        .build(&mut buffer)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::font::definition::{FontPackMetadata, FontStyle, FontWeight};

    use super::*;

    #[tokio::test]
    async fn generate_example() {
        let pack = FontPackDefinition {
            metadata: FontPackMetadata {
                family_name: "Family Name".to_string(),
                description: "Description".to_string(),
                code_page: "ASCII".to_string(),
                ..Default::default()
            },
            fonts: vec!["test".into()],
        };

        let font = FontDefinition {
            version: 0,
            height: 6,
            // This is only used to load `FontGlyphs`
            // We can skip this
            glyphs: vec![],
            italic_space_adjust: 6,
            space_above: 4,
            space_below: 5,
            weight: Some(FontWeight::Normal),
            style: FontStyle {
                serif: true,
                oblique: false,
                italic: true,
                monospaced: false,
            },
            cap_height: 2,
            x_height: 7,
            baseline_height: 1,
        };

        let mut font_glyphs = FontGlyphs::default();
        font_glyphs.insert(b'a', 3, vec![0, 1, 2, 3, 4, 5]);
        font_glyphs.insert(b'c', 8, vec![255, 255, 255, 255, 255, 255]);

        let mut buffer = Cursor::new(Vec::new());
        generate_serial_builder(pack, vec![(font, font_glyphs)])
            .unwrap()
            .build(&mut buffer)
            .await
            .unwrap();

        let expected = [
            b"FONTPACK".iter(),
            // Metadata pointer
            [15, 0, 0].iter(),
            // Fount count
            [1].iter(),
            // `test` font pointer
            [66, 0, 0].iter(),
            // Metadata length
            [21, 0, 0].iter(),
            // Family name
            [36, 0, 0].iter(),
            // Author
            [0, 0, 0].iter(),
            // Copyright
            [0, 0, 0].iter(),
            // Description
            [48, 0, 0].iter(),
            // Version
            [0, 0, 0].iter(),
            // Code page
            [60, 0, 0].iter(),
            b"Family Name\x00".iter(),
            b"Description\x00".iter(),
            b"ASCII\x00".iter(),
            [
                0,    // Font version
                6,    // Font height
                3,    // Total glyphs
                b'a', // First glyph
            ]
            .iter(),
            // Widths offset
            [16, 0].iter(),
            // Bitmap table offset
            [19, 0].iter(),
            [
                6,           // Italic space adjust
                4,           // Space above
                5,           // Space below
                0x80,        // Weight
                0b0000_0101, // Style
                2,           // Cap height
                7,           // X height
                1,           // Baseline height
            ]
            .iter(),
            // Widths
            [3, 0, 8].iter(),
            // Bitmap table
            // First glyph
            [25, 0].iter(),
            // Unused glyph
            [0, 0].iter(),
            // Second glyph
            [31, 0].iter(),
            // First glyph bitmap
            [0, 1, 2, 3, 4, 5].iter(),
            // Second glyph bitmap
            [255, 255, 255, 255, 255, 255].iter(),
        ]
        .into_iter()
        .flatten()
        .copied()
        .collect::<Vec<_>>();

        assert_eq!(
            buffer.get_ref().clone(),
            expected,
            "Generated:\n{}\n\nExpected:\n{}",
            buffer.get_ref().escape_ascii(),
            expected.escape_ascii()
        );
    }
}
