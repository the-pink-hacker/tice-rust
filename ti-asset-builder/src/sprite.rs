use std::path::Path;

use anyhow::Context;
use futures::{StreamExt, TryStreamExt};

use crate::{
    cli::CliSpriteCommand,
    image::{Color8, RawImage},
    output::OutputType,
    path::PathExt,
    sprite::definition::{Sprite, SpriteTableDefinition},
};

mod definition;
mod output;

async fn load_sprite_definition(path: &Path) -> anyhow::Result<SpriteTableDefinition> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read sprite table at {path:?}"))?;
    let definition = toml::from_str(&raw)
        .with_context(|| format!("Failed to parse sprite table at {path:?}"))?;

    Ok(definition)
}

impl RawImage {
    async fn load_sprite(
        sprite_table_path: &Path,
        name: String,
        sprite: Sprite,
    ) -> anyhow::Result<RawSprite> {
        let expanded_sprite = sprite.to_expanded();
        let image_path = sprite_table_path.relative_parent_suffix(&expanded_sprite.path, ".png")?;
        let (width, height, pixels_raw) = Self::load(&image_path)
            .await?
            .rotate(expanded_sprite.rotation)
            .into_rgb24();

        let pixels = pixels_raw.into_iter().map(Color8::from).collect();

        Ok(RawSprite {
            name,
            pixels,
            width: width as u8,
            height: height as u8,
        })
    }
}

impl SpriteTableDefinition {
    async fn load_sprites(self, sprite_table_path: &Path) -> anyhow::Result<Vec<RawSprite>> {
        futures::stream::iter(
            self.sprites
                .into_iter()
                .map(|(name, sprite)| RawImage::load_sprite(sprite_table_path, name, sprite)),
        )
        .buffered(std::thread::available_parallelism()?.into())
        .try_collect()
        .await
    }
}

#[derive(Debug)]
struct RawSprite {
    name: String,
    pixels: Vec<Color8>,
    width: u8,
    height: u8,
}

pub async fn build(command: CliSpriteCommand) -> anyhow::Result<()> {
    let definition_path = command.definition.canonicalize().with_context(|| {
        format!(
            "Failed to get canon sprite table path: {:?}",
            command.definition
        )
    })?;

    tokio::fs::create_dir_all(&command.output).await?;

    let name = definition_path
        .file_stem()
        .with_context(|| format!("Cannot get file name of definition: {definition_path:?}"))?
        .to_str()
        .with_context(|| format!("Invalid definition name: {definition_path:?}"))?;

    let sprites = load_sprite_definition(&definition_path)
        .await?
        .load_sprites(&definition_path)
        .await?;

    match command.output_type {
        OutputType::Binary => {
            output::bin::generate_sprite_file_bin(sprites, name, &command.output).await?
        }
        OutputType::C => output::c::generate_sprite_file_c(sprites, name, &command.output).await?,
        OutputType::Assembly => unimplemented!(),
    }

    Ok(())
}
