pub mod definition;

use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::{
    cli::CliFontPackCommand,
    font::definition::{
        FontDefinition, FontDefinitionWrapper, FontPackDefinition, FontPackDefinitionWrapper,
    },
    path::PathExt,
};

fn load_pack_definition(path: &Path) -> anyhow::Result<FontPackDefinition> {
    let raw = std::fs::read_to_string(path)
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

fn load_font_definition(path: &Path) -> anyhow::Result<FontDefinition> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read font definition at {path:?}"))?;
    let definition = toml::from_str::<FontDefinitionWrapper>(&raw)
        .with_context(|| format!("Failed to parse font definition at {path:?}"))?
        .font;
    Ok(definition)
}

pub fn build(command: CliFontPackCommand) -> anyhow::Result<()> {
    let pack_definition_path = command.definition.canonicalize().with_context(|| {
        format!(
            "Failed to get canon font pack definition path: {:?}",
            command.definition
        )
    })?;
    let pack_definition = load_pack_definition(&command.definition)?;

    Ok(())
}
