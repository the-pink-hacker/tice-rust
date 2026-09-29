use std::path::Path;

use linked_hash_map::LinkedHashMap;
use serseg::builder::{SerialBuilder, SerialSectorBuilder};

use crate::sprite::{RawImage, definition::Sprite};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SectorId {
    Sprite(usize),
}

pub async fn generate_sprite_file_bin(
    sprite_table_path: &Path,
    out_path: &Path,
    sprite_collection_name: &str,
    collection: LinkedHashMap<String, Sprite>,
) -> anyhow::Result<()> {
    let mut builder = SerialBuilder::<SectorId>::default();

    for (i, (_, sprite)) in collection.into_iter().enumerate() {
        let (width, height, pixels) = RawImage::load_sprite(sprite_table_path, sprite).await?;

        builder = builder.sector(
            SectorId::Sprite(i),
            SerialSectorBuilder::default()
                .u8(width)
                .u8(height)
                .bytes(pixels.into_iter().map(u8::from)),
        );
    }

    let file_path = out_path.join(format!("{sprite_collection_name}.bin"));
    let file = tokio::fs::File::create(file_path).await?;
    let mut buffer = tokio::io::BufWriter::new(file);
    builder.build(&mut buffer).await?;

    Ok(())
}
