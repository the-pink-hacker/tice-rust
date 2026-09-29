use std::{collections::HashMap, path::Path};

use anyhow::Context;
use image::GenericImageView;
use imageproc::geometric_transformations::Interpolation;
use linked_hash_map::LinkedHashMap;

use crate::{cli::CliSpriteCommand, output::OutputType, path::PathExt, sprite::definition::Sprite};

mod definition;
mod output;

#[derive(Debug, Clone, Copy)]
pub struct ColorRGB24 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl From<(u8, u8, u8)> for ColorRGB24 {
    fn from(value: (u8, u8, u8)) -> Self {
        let (red, green, blue) = value;
        Self { red, green, blue }
    }
}

impl From<ColorRGB24> for (u8, u8, u8) {
    fn from(value: ColorRGB24) -> Self {
        (value.red, value.green, value.blue)
    }
}

impl From<[u8; 3]> for ColorRGB24 {
    fn from(value: [u8; 3]) -> Self {
        let [red, green, blue] = value;
        Self { red, green, blue }
    }
}

impl From<ColorRGB24> for [u8; 3] {
    fn from(value: ColorRGB24) -> Self {
        [value.red, value.green, value.blue]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ColorMonochrome(bool);

impl From<ColorMonochrome> for bool {
    fn from(value: ColorMonochrome) -> Self {
        value.0
    }
}

impl From<bool> for ColorMonochrome {
    fn from(value: bool) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Color8(u8);

impl From<u8> for Color8 {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<Color8> for u8 {
    fn from(value: Color8) -> Self {
        value.0
    }
}

impl From<ColorRGB24> for Color8 {
    fn from(value: ColorRGB24) -> Self {
        let (red, green, blue) = value.into();
        let red = (red / 32) << 5;
        let green = green / 32;
        let blue = (blue / 64) << 3;
        Self(red | green | blue)
    }
}

pub struct RawImage {
    image: image::DynamicImage,
}

impl RawImage {
    pub async fn load(path: &Path) -> anyhow::Result<Self> {
        let file = tokio::fs::read(path)
            .await
            .with_context(|| format!("Failed to read image file at: {path:?}"))?;

        let image = image::load_from_memory_with_format(&file, image::ImageFormat::Png)
            .with_context(|| format!("Failed to parse PNG: {path:?}"))?;

        Ok(Self { image })
    }

    pub async fn load_sprite(
        sprite_table_path: &Path,
        sprite: Sprite,
    ) -> anyhow::Result<(u8, u8, Vec<Color8>)> {
        let expanded_sprite = sprite.to_expanded();
        let image_path = sprite_table_path.relative_parent_suffix(&expanded_sprite.path, ".png")?;
        let (width, height, pixels_raw) = Self::load(&image_path)
            .await?
            .rotate(expanded_sprite.rotation)
            .into_rgb24();

        let pixels = pixels_raw.into_iter().map(Color8::from).collect();

        Ok((width as u8, height as u8, pixels))
    }

    pub fn rotate(mut self, degrees: f32) -> Self {
        if (degrees % 360.0).is_normal() {
            self.image = imageproc::geometric_transformations::rotate_about_center(
                &self.image.to_rgb8(),
                degrees.to_radians(),
                Interpolation::Bilinear,
                imageproc::geometric_transformations::Border::Constant(image::Rgb([0, 0, 0])),
            )
            .into();

            self
        } else {
            self
        }
    }

    /// Returns the width, height, and pixel data of the image
    pub fn into_rgb24(self) -> (u32, u32, Vec<ColorRGB24>) {
        let (width, height) = self.image.dimensions();
        let pixels = self
            .image
            .into_rgb8()
            .pixels()
            .map(|pixel| pixel.0.into())
            .collect();

        (width, height, pixels)
    }

    /// Returns the width, height, and pixel data of the image
    pub fn into_monochrome(self) -> (u32, u32, Vec<ColorMonochrome>) {
        let (width, height) = self.image.dimensions();
        let pixels = self
            .image
            .into_luma_alpha8()
            .pixels()
            .map(|pixel| ColorMonochrome(pixel.0[1] != 0))
            .collect();

        (width, height, pixels)
    }
}

async fn load_sprite_definition(
    path: &Path,
) -> anyhow::Result<HashMap<String, LinkedHashMap<String, Sprite>>> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read sprite table at {path:?}"))?;
    let definition = toml::from_str(&raw)
        .with_context(|| format!("Failed to parse sprite table at {path:?}"))?;

    Ok(definition)
}

#[derive(Debug)]
struct RawSprite {
    pixels: Vec<Color8>,
    width: u8,
}

impl RawSprite {
    fn new(pixels: Vec<Color8>, width: u8) -> Self {
        Self { pixels, width }
    }
}

pub async fn build(command: CliSpriteCommand) -> anyhow::Result<()> {
    let definition_path = command.definition.canonicalize().with_context(|| {
        format!(
            "Failed to get canon sprite table path: {:?}",
            command.definition
        )
    })?;

    let definition = load_sprite_definition(&definition_path).await?;
    tokio::fs::create_dir_all(&command.output).await?;

    for (sprite_collection_name, collection) in definition.into_iter() {
        match command.output_type {
            OutputType::Binary => {
                output::bin::generate_sprite_file_bin(
                    &command.definition,
                    &command.output,
                    &sprite_collection_name,
                    collection,
                )
                .await?
            }
            OutputType::C => {
                output::c::generate_sprite_file_c(
                    &command.definition,
                    &command.output,
                    &sprite_collection_name,
                    collection,
                )
                .await?
            }
            OutputType::Assembly => unimplemented!(),
        }
    }

    Ok(())
}
