use std::path::PathBuf;

use serde::Deserialize;
use serde_valid::Validate;

#[derive(Debug, Deserialize, Validate, Clone, Default)]
pub struct ExpandedSprite {
    pub path: PathBuf,
    #[validate(minimum = -360.0)]
    #[validate(maximum = 360.0)]
    #[serde(default)]
    pub rotation: f32,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Sprite {
    Path(PathBuf),
    Expanded(ExpandedSprite),
}

impl Sprite {
    pub fn to_expanded(&self) -> ExpandedSprite {
        match self {
            Self::Path(path) => ExpandedSprite {
                path: path.to_path_buf(),
                ..Default::default()
            },
            Self::Expanded(sprite) => sprite.clone(),
        }
    }
}
