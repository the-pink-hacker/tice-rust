use std::path::Path;

use anyhow::Context;
use futures::{StreamExt, TryStreamExt};

use crate::{
    cli::CliSpriteCommand,
    image::{Color8, RawImage},
    output::OutputType,
    palette::{PaletteDefinition, PaletteLookup},
    path::PathExt,
    sprite::definition::{Sprite, SpriteTableDefinition},
};

pub mod definition;
mod output;

impl SpriteTableDefinition {
    pub async fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read sprite table at {path:?}"))?;
        toml::from_str(&raw).with_context(|| format!("Failed to parse sprite table at {path:?}"))
    }

    pub async fn load_images(self, sprite_table_path: &Path) -> anyhow::Result<Vec<RawImage>> {
        futures::stream::iter(
            self.sprites
                .into_values()
                .map(|sprite| RawImage::load_from_sprite(sprite_table_path, sprite)),
        )
        .buffered(std::thread::available_parallelism()?.into())
        .try_collect()
        .await
    }

    async fn load_sprites(
        self,
        sprite_table_path: &Path,
        palette_lookup: Option<PaletteLookup>,
    ) -> anyhow::Result<Vec<RawSprite>> {
        futures::stream::iter(self.sprites.into_iter().map(|(name, sprite)| async {
            RawImage::load_from_sprite(sprite_table_path, sprite)
                .await?
                .into_raw_sprite(name, palette_lookup.as_ref())
        }))
        .buffered(std::thread::available_parallelism()?.into())
        .try_collect()
        .await
    }
}

impl RawImage {
    async fn load_from_sprite(sprite_table_path: &Path, sprite: Sprite) -> anyhow::Result<Self> {
        let expanded_sprite = sprite.into_expanded();
        let image_path = sprite_table_path.relative_parent_suffix(&expanded_sprite.path, ".png")?;
        let mut image = Self::load(&image_path)
            .await
            .with_context(|| format!("Failed to load sprite: {image_path:?}"))?;

        image.rotate(expanded_sprite.rotation);

        Ok(image)
    }

    fn into_raw_sprite(
        self,
        name: String,
        palette_lookup: Option<&PaletteLookup>,
    ) -> anyhow::Result<RawSprite> {
        let (width, height, pixels_raw) = self.into_rgba24();

        let pixels_iter = pixels_raw.into_iter();

        let pixels = if let Some(palette_lookup) = palette_lookup {
            pixels_iter
                .map(|pixel| palette_lookup.lookup(pixel))
                .try_collect()?
        } else {
            pixels_iter.map(Color8::from).map(u8::from).collect()
        };

        Ok(RawSprite {
            name,
            pixels,
            width: width as u8,
            height: height as u8,
        })
    }
}

#[derive(Debug)]
struct RawSprite {
    name: String,
    pixels: Vec<u8>,
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

    let palette_lookup = if let Some(palette) = command.palette {
        Some(PaletteDefinition::load(&palette).await?.get_lookup()?)
    } else {
        None
    };

    let sprites = SpriteTableDefinition::load(&definition_path)
        .await?
        .load_sprites(&definition_path, palette_lookup)
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
