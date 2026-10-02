use std::path::Path;

use anyhow::Context;

#[derive(Debug, Clone, Copy)]
pub struct ColorRGB24 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl From<[u8; 4]> for ColorRGB24 {
    fn from(value: [u8; 4]) -> Self {
        let [red, green, blue, _alpha] = value;
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

    pub fn rotate(mut self, degrees: f32) -> Self {
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

        self
    }

    /// Returns the width, height, and pixel data of the image
    pub fn into_rgb24(self) -> (u32, u32, Vec<ColorRGB24>) {
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
