use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

use serde::Deserialize;
use serde_valid::Validate;

use crate::image::ColorRGBA32;

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SpriteEncodingFormat {
    #[default]
    LibC,
    RunLength,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct SpriteEncoding {
    pub format: SpriteEncodingFormat,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct SpritePalette {
    pub reserve: HashMap<String, ColorRGBA32>,
}

#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default)]
pub struct SpriteTableDefinition {
    pub encoding: SpriteEncoding,
    pub palette: SpritePalette,
    pub sprites: BTreeMap<String, Sprite>,
}

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct ExpandedSprite {
    pub path: PathBuf,
    #[validate(minimum = -360.0)]
    #[validate(maximum = 360.0)]
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub encoding: Option<SpriteEncoding>,
}

impl From<PathBuf> for ExpandedSprite {
    fn from(value: PathBuf) -> Self {
        Self {
            path: value,
            rotation: 0.0,
            encoding: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum Sprite {
    Path(PathBuf),
    Expanded(ExpandedSprite),
}

impl Sprite {
    pub fn into_expanded(self) -> ExpandedSprite {
        match self {
            Self::Path(path) => path.into(),
            Self::Expanded(sprite) => sprite,
        }
    }
}
