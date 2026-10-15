use std::path::Path;

use serseg::builder::{SerialBuilder, SerialSectorBuilder};

use crate::sprite::RawSprite;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SectorId {
    LookupTable,
    Sprite(usize),
}

pub async fn generate_sprite_file_bin(
    sprites: Vec<RawSprite>,
    lookup_table: bool,
    name: &str,
    out_path: &Path,
) -> anyhow::Result<()> {
    let mut builder = SerialBuilder::<SectorId>::default();

    if lookup_table {
        let mut lookup_builder = SerialSectorBuilder::default();

        for i in 0..sprites.len() {
            lookup_builder =
                lookup_builder.dynamic_u24(SectorId::LookupTable, SectorId::Sprite(i), 0);
        }

        builder = builder.sector(SectorId::LookupTable, lookup_builder);
    }

    for (i, sprite) in sprites.into_iter().enumerate() {
        builder = builder.sector(
            SectorId::Sprite(i),
            SerialSectorBuilder::default()
                .u8(sprite.width)
                .u8(sprite.height)
                .bytes(sprite.pixels),
        );
    }

    let file_path = out_path.join(format!("{name}.bin"));
    let file = tokio::fs::File::create(file_path).await?;
    let mut buffer = tokio::io::BufWriter::new(file);
    builder.build(&mut buffer).await?;

    Ok(())
}
