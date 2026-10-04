use std::{
    collections::{BTreeSet, HashMap},
    path::Path,
};

use anyhow::Context;
use futures::{StreamExt, TryStreamExt};
use log::info;
use serde::{Deserialize, Serialize};

use crate::{
    cli::CliPaletteCommand,
    image::{ColorRGB1555, ColorRGBA32},
    sprite::definition::SpriteTableDefinition,
};

#[derive(Debug)]
pub struct PaletteLookup {
    colors: HashMap<ColorRGB1555, u8>,
    reserve: HashMap<ColorRGBA32, u8>,
}

impl PaletteLookup {
    pub fn lookup(&self, color: ColorRGBA32) -> anyhow::Result<u8> {
        self.reserve
            .get(&color)
            .or_else(|| self.colors.get(&color.into()))
            .cloned()
            .with_context(|| format!("Color is missing from palette: {color:#?}"))
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct PaletteDefinition {
    #[serde(with = "base64")]
    colors: Vec<ColorRGB1555>,
    reserve: Vec<ColorRGBA32>,
}

// https://users.rust-lang.org/t/serialize-a-vec-u8-to-json-as-base64/57781/2
mod base64 {
    use base64::prelude::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::image::ColorRGB1555;

    pub fn serialize<S: Serializer>(v: &Vec<ColorRGB1555>, s: S) -> Result<S::Ok, S::Error> {
        let bytes = v
            .iter()
            .cloned()
            .map(u16::from)
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        let base64 = BASE64_STANDARD.encode(bytes);
        String::serialize(&base64, s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<ColorRGB1555>, D::Error> {
        let base64 = String::deserialize(d)?;
        let bytes = BASE64_STANDARD
            .decode(base64.as_bytes())
            .map_err(serde::de::Error::custom)?;

        // Truncates incomplete values

        let colors = bytes
            .into_chunks::<2>()
            .into_iter()
            .map(u16::from_le_bytes)
            .map(ColorRGB1555::from)
            .collect();

        Ok(colors)
    }
}

impl PaletteDefinition {
    fn new(colors: BTreeSet<ColorRGB1555>, reserve: BTreeSet<ColorRGBA32>) -> Self {
        Self {
            colors: colors.into_iter().collect(),
            reserve: reserve.into_iter().collect(),
        }
    }

    async fn write(&self, path: &Path) -> anyhow::Result<()> {
        let raw = toml::to_string(&self)
            .with_context(|| format!("Failed to serialize definition: {self:?}"))?;

        tokio::fs::write(path, raw)
            .await
            .with_context(|| format!("Failed to write palette definition: {path:?}"))
    }

    async fn write_c_files(&self, folder: &Path) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(folder).await?;

        let count = self.colors.len();
        let count = u8::try_from(count).map_err(|_| palette_full_error(count))?;
        Ok(())
    }

    pub async fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to load palette definition: {path:?}"))?;

        toml::from_str(&raw)
            .with_context(|| format!("Failed to deserialize palette definition: {path:?}"))
    }

    pub fn get_lookup(self) -> anyhow::Result<PaletteLookup> {
        let reserve_count = self.reserve.len();
        let count = self.colors.len() + reserve_count;
        u8::try_from(count).map_err(|_| palette_full_error(count))?;

        // 1 because 0 is reserved for transparent
        let reserve = self.reserve.into_iter().zip(1..).collect();
        let colors = self
            .colors
            .into_iter()
            .zip((reserve_count as u8 + 1)..)
            .collect();

        Ok(PaletteLookup { colors, reserve })
    }
}

fn log_color_count(count: usize) {
    info!("Palette colors usage: {count:>3}/256");
}

fn palette_full_error(count: usize) -> anyhow::Error {
    anyhow::anyhow!("Palette is full: {count} > 255")
}

pub async fn build(command: CliPaletteCommand) -> anyhow::Result<()> {
    let definitions = futures::stream::iter(command.definitions.into_iter().map(|path| async {
        SpriteTableDefinition::load(&path)
            .await
            .map(|definition| (path, definition))
    }))
    .buffered(std::thread::available_parallelism()?.into())
    .try_collect::<Vec<_>>()
    .await?;

    let mut colors = BTreeSet::new();
    let mut reserve = BTreeSet::new();

    // Populate reserve table
    definitions.iter().for_each(|(_, definition)| {
        definition
            .palette
            .reserve
            .iter()
            .cloned()
            .for_each(|color| {
                reserve.insert(color);
            })
    });

    for (definition_path, definition) in definitions {
        info!("Adding sprites at: {definition_path:?}");
        let (images, palette) = definition.load_images(&definition_path).await?;
        images
            .iter()
            .for_each(|i| i.collect_colors(&mut colors, &palette));

        let count = colors.len();

        u8::try_from(count).map_err(|_| palette_full_error(count))?;

        log_color_count(count);
    }

    info!("Palette is completed.");

    log_color_count(colors.len());

    let definition = PaletteDefinition::new(colors, reserve);

    definition.write(&command.output).await?;
    definition.write_c_files(&command.output_c_folder).await?;

    Ok(())
}
