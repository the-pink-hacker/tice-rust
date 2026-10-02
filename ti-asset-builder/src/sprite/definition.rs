use std::path::PathBuf;

use indexmap::IndexMap;
use serde::Deserialize;
use serde_valid::Validate;

#[derive(Debug, Deserialize, Clone)]
pub struct SpriteTableDefinition {
    pub sprites: IndexMap<String, Sprite>,
}

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct ExpandedSprite {
    pub path: PathBuf,
    #[validate(minimum = -360.0)]
    #[validate(maximum = 360.0)]
    #[serde(default)]
    pub rotation: f32,
}

impl From<PathBuf> for ExpandedSprite {
    fn from(value: PathBuf) -> Self {
        Self {
            path: value,
            rotation: 0.0,
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
    pub fn to_expanded(&self) -> ExpandedSprite {
        match self {
            Self::Path(path) => path.to_path_buf().into(),
            Self::Expanded(sprite) => sprite.clone(),
        }
    }
}
