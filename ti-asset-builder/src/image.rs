use std::{
    collections::{BTreeSet, HashSet},
    path::Path,
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(transparent)]
pub struct ColorRGB1555(u16);

impl From<ColorRGBA32> for ColorRGB1555 {
    fn from(value: ColorRGBA32) -> Self {
        if value.is_transparent() {
            Self::default()
        } else {
            let ColorRGBA32 {
                red,
                green,
                blue,
                alpha: _,
            } = value;

            let red = ((red / 8) as u16) << 10;
            let green = ((green / 8) as u16) << 5;
            let blue = (blue / 8) as u16;

            Self(red | green | blue)
        }
    }
}

impl From<u16> for ColorRGB1555 {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<ColorRGB1555> for u16 {
    fn from(value: ColorRGB1555) -> Self {
        value.0
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ColorRGBA32 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    #[serde(default = "ColorRGBA32::default_transparency")]
    #[serde(skip_serializing_if = "ColorRGBA32::is_alpha_default")]
    pub alpha: u8,
}

impl ColorRGBA32 {
    pub fn is_transparent(&self) -> bool {
        self.alpha == 0
    }

    fn default_transparency() -> u8 {
        u8::MAX
    }

    fn is_alpha_default(alpha: &u8) -> bool {
        *alpha == u8::MAX
    }
}

impl From<[u8; 4]> for ColorRGBA32 {
    fn from(value: [u8; 4]) -> Self {
        let [red, green, blue, alpha] = value;
        Self {
            red,
            green,
            blue,
            alpha,
        }
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

#[derive(Debug, Default, Clone, Copy)]
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

impl From<ColorRGBA32> for Color8 {
    fn from(value: ColorRGBA32) -> Self {
        if value.is_transparent() {
            Self::default()
        } else {
            let ColorRGBA32 {
                red,
                green,
                blue,
                alpha: _,
            } = value;

            let red = (red / 32) << 5;
            let green = green / 32;
            let blue = (blue / 64) << 3;

            Self(red | green | blue)
        }
    }
}

pub struct RawImage {
    image: image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
}

impl RawImage {
    pub async fn load(path: &Path) -> anyhow::Result<Self> {
        let file = tokio::fs::read(path)
            .await
            .with_context(|| format!("Failed to read image file at: {path:?}"))?;

        let image = image::load_from_memory_with_format(&file, image::ImageFormat::Png)
            .with_context(|| format!("Failed to parse PNG: {path:?}"))?
            .into_rgba8();

        Ok(Self { image })
    }

    pub fn rotate(&mut self, degrees: f32) {
        use imageproc::geometric_transformations as img;
        let wrapped_degrees = degrees % 360.0;

        if wrapped_degrees.is_normal() {
            match wrapped_degrees {
                90.0 => {
                    self.image = img::rotate90(&self.image);
                }
                180.0 => {
                    img::rotate180_mut(&mut self.image);
                }
                270.0 => {
                    self.image = img::rotate270(&self.image);
                }
                _ => {
                    self.image = img::rotate_about_center(
                        &self.image,
                        degrees.to_radians(),
                        img::Interpolation::Bilinear,
                        img::Border::Constant(image::Rgba(Default::default())),
                    );
                }
            }
        }
    }

    pub fn collect_colors(
        &self,
        colors: &mut BTreeSet<ColorRGB1555>,
        reserve_palette: &HashSet<ColorRGBA32>,
    ) {
        self.image
            .pixels()
            .map(|pixel| ColorRGBA32::from(pixel.0))
            .filter(|color| !color.is_transparent())
            .filter(|color| !reserve_palette.contains(color))
            .map(ColorRGB1555::from)
            .for_each(|color| {
                colors.insert(color);
            });
    }

    /// Returns the width, height, and pixel data of the image
    pub fn into_rgba24(self) -> (u32, u32, Vec<ColorRGBA32>) {
        let (width, height) = self.image.dimensions();
        let pixels = self.image.pixels().map(|pixel| pixel.0.into()).collect();

        (width, height, pixels)
    }

    /// Returns the width, height, and pixel data of the image
    pub fn into_monochrome(self) -> (u32, u32, Vec<ColorMonochrome>) {
        let (width, height) = self.image.dimensions();
        let pixels = image::DynamicImage::from(self.image)
            .to_luma_alpha8()
            .pixels()
            .map(|pixel| ColorMonochrome(pixel.0[1] != 0))
            .collect();

        (width, height, pixels)
    }
}
