use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
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
    reserve: HashMap<ColorRGBA32, (String, u8)>,
}

impl PaletteLookup {
    pub fn lookup(&self, color: ColorRGBA32) -> anyhow::Result<u8> {
        if color.is_transparent() {
            Ok(0)
        } else {
            self.reserve
                .get(&color)
                .map(|(_, value)| value)
                .or_else(|| self.colors.get(&color.into()))
                .cloned()
                .with_context(|| format!("Color is missing from palette: {color:#?}"))
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct PaletteDefinition {
    #[serde(with = "base64")]
    colors: Vec<ColorRGB1555>,
    reserve: BTreeMap<String, ColorRGBA32>,
}

// https://users.rust-lang.org/t/serialize-a-vec-u8-to-json-as-base64/57781/2
mod base64 {
    use base64::prelude::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use crate::image::ColorRGB1555;

    pub fn serialize<S: Serializer>(v: &[ColorRGB1555], s: S) -> Result<S::Ok, S::Error> {
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
    fn new(colors: BTreeSet<ColorRGB1555>, reserve: BTreeMap<String, ColorRGBA32>) -> Self {
        Self {
            colors: colors.into_iter().collect(),
            reserve,
        }
    }

    async fn write(&self, path: &Path) -> anyhow::Result<()> {
        let raw = toml::to_string(&self)
            .with_context(|| format!("Failed to serialize definition: {self:?}"))?;

        tokio::fs::write(path, raw)
            .await
            .with_context(|| format!("Failed to write palette definition: {path:?}"))
    }

    fn get_color_count(&self) -> anyhow::Result<u8> {
        let count = self.reserve.len() + self.colors.len() + 1;
        u8::try_from(count).map_err(|_| palette_full_error(count))
    }

    async fn write_c_files(self, folder: &Path) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(folder).await?;

        let file_stem = "palette";

        let mut header_out = String::from("#include <stdint.h>\n\n");
        let mut c_out = format!("#include \"{file_stem}.h\"\n\n");

        let count = self.get_color_count()?;

        header_out += &format!("#define PALETTE_COUNT {count}\n");
        header_out += &format!("#define PALETTE_SIZE {}\n\n", count * 2);

        for (name, i) in self.reserve.keys().zip(1u8..) {
            header_out += &format!("#define PALETTE_RESERVE_{} {i}\n", name.to_uppercase());
        }

        header_out += &format!("extern const uint16_t PALETTE_TABLE[{count}];\n");
        c_out += &format!("const uint16_t PALETTE_TABLE[{count}] = {{");

        c_out += &[ColorRGB1555::default()]
            .into_iter()
            .chain(self.reserve.into_values().map(ColorRGB1555::from))
            .chain(self.colors)
            .map(u16::from)
            .map(|x| format!("{x}"))
            .intersperse(String::from(","))
            .collect::<String>();

        c_out += "};";

        let c_path = folder.join(format!("{file_stem}.c"));
        let header_path = folder.join(format!("{file_stem}.h"));

        tokio::fs::write(&header_path, header_out)
            .await
            .with_context(|| format!("Failed to write c file: {header_path:?}"))?;

        tokio::fs::write(&c_path, c_out)
            .await
            .with_context(|| format!("Failed to write c file: {c_path:?}"))
    }

    pub async fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to load palette definition: {path:?}"))?;

        toml::from_str(&raw)
            .with_context(|| format!("Failed to deserialize palette definition: {path:?}"))
    }

    pub fn get_lookup(self) -> anyhow::Result<PaletteLookup> {
        let colors = self
            .colors
            .into_iter()
            .zip((self.reserve.len() as u8 + 1)..)
            .collect();

        // 1 because 0 is reserved for transparent
        let reserve = self
            .reserve
            .into_iter()
            .zip(1..)
            .map(|((name, color), index)| (color, (name, index)))
            .collect();

        Ok(PaletteLookup { colors, reserve })
    }
}

fn log_color_count(count: usize) {
    info!("Palette colors usage: {count:>3}/256");
}

fn palette_full_error(count: usize) -> anyhow::Error {
    anyhow::anyhow!("Palette is full: {count} > 256")
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
    let mut reserve = BTreeMap::new();

    // Populate reserve table
    definitions.iter().for_each(|(_, definition)| {
        definition.palette.reserve.iter().for_each(|(name, color)| {
            reserve.insert(name.clone(), *color);
        })
    });

    let reserve_set = reserve.values().cloned().collect();

    for (definition_path, definition) in definitions {
        info!("Adding sprites at: {definition_path:?}");

        definition
            .load_images(&definition_path)
            .await?
            .iter()
            .for_each(|i| i.collect_colors(&mut colors, &reserve_set));

        let count = colors.len();

        u8::try_from(count).map_err(|_| palette_full_error(count))?;

        log_color_count(count);
    }

    info!("Palette is completed.");

    log_color_count(colors.len() + reserve.len() + 1);

    let definition = PaletteDefinition::new(colors, reserve);

    definition.write(&command.output).await?;
    definition.write_c_files(&command.output_c_folder).await?;

    Ok(())
}
