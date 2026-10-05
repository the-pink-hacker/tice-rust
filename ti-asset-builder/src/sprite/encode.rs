use log::debug;

use crate::sprite::{
    RawSprite,
    definition::{SpriteEncoding, SpriteEncodingFormat},
};

pub trait SpriteEncoder {
    fn encode(&self, sprite: RawSprite) -> anyhow::Result<RawSprite>;
}

pub struct EncoderLibC;

impl SpriteEncoder for EncoderLibC {
    fn encode(&self, sprite: RawSprite) -> anyhow::Result<RawSprite> {
        Ok(sprite)
    }
}

pub struct EncoderRunLength;

const RUN_LENGTH_MASK: u8 = 0b1000_0000;

impl SpriteEncoder for EncoderRunLength {
    fn encode(&self, mut sprite: RawSprite) -> anyhow::Result<RawSprite> {
        debug!("Encoding run length sprite: {}", sprite.name);
        let pixel_count = sprite.pixels.len();
        let mut output = Vec::with_capacity(pixel_count);
        sprite
            .pixels
            .chunks_exact(sprite.width as usize)
            .try_for_each(|line| {
                // Validate colors fit in 7 bits
                line.iter().try_for_each(|&pixel| {
                    if pixel < RUN_LENGTH_MASK {
                        Ok(())
                    } else {
                        Err(anyhow::anyhow!("Color exceeded 7-bits in {}", sprite.name))
                    }
                })?;

                let mut line = line.iter().cloned().peekable();

                while let Some(current_pixel) = line.next() {
                    if let Some(&next_pixel) = line.peek()
                        && next_pixel == current_pixel
                    {
                        line.next();
                        let mut count = 1u8;

                        while line
                            .next_if(|&pixel| pixel == current_pixel && count < RUN_LENGTH_MASK - 1)
                            .is_some()
                        {
                            count += 1;
                        }

                        output.push(RUN_LENGTH_MASK | count);
                        output.push(current_pixel);
                    } else {
                        output.push(current_pixel);
                    }
                }

                output.push(u8::MAX);

                Ok::<_, anyhow::Error>(())
            })?;

        sprite.pixels = output;

        let length = sprite.pixels.len();
        let percent = (length as f32 / pixel_count as f32) * 100.0;

        debug!("Sprite size: {length}/{pixel_count} ({percent})");

        Ok(sprite)
    }
}

impl SpriteEncoder for SpriteEncoding {
    fn encode(&self, sprite: RawSprite) -> anyhow::Result<RawSprite> {
        match self.format {
            SpriteEncodingFormat::LibC => EncoderLibC.encode(sprite),
            SpriteEncodingFormat::RunLength => EncoderRunLength.encode(sprite),
        }
    }
}
