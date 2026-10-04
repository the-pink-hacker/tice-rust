use std::path::Path;

use serseg::builder::{SerialBuilder, SerialSectorBuilder};

use crate::sprite::RawSprite;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SectorId {
    Sprite(usize),
}

pub async fn generate_sprite_file_bin(
    sprites: Vec<RawSprite>,
    name: &str,
    out_path: &Path,
) -> anyhow::Result<()> {
    let mut builder = SerialBuilder::<SectorId>::default();

    for (i, sprite) in sprites.into_iter().enumerate() {
        builder = builder.sector(
            SectorId::Sprite(i),
            SerialSectorBuilder::default()
                .u8(sprite.width)
                .u8(sprite.height)
                .bytes(sprite.pixels.into_iter()),
        );
    }

    let file_path = out_path.join(format!("{name}.bin"));
    let file = tokio::fs::File::create(file_path).await?;
    let mut buffer = tokio::io::BufWriter::new(file);
    builder.build(&mut buffer).await?;

    Ok(())
}
