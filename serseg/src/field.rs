use std::{hash::Hash, io::SeekFrom, path::PathBuf};

use anyhow::{Context, bail};
use indexmap::IndexMap;
use tokio::io::{AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
use u24::u24;

use crate::{prelude::*, tracker::SerialTracker};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ScaleRounding {
    Ceiling,
    Nearest,
    #[default]
    Floor,
}

impl ScaleRounding {
    const fn apply(&self, value: usize, scale: usize) -> usize {
        match self {
            Self::Ceiling => value.div_ceil(scale),
            Self::Nearest => {
                if value.is_multiple_of(2) {
                    value / scale
                } else {
                    (value + 1) / scale
                }
            }
            Self::Floor => value / scale,
        }
    }
}

pub trait Scale {
    fn get(self) -> (ScaleRounding, usize);
}

impl Scale for usize {
    #[inline]
    fn get(self) -> (ScaleRounding, usize) {
        (ScaleRounding::default(), self)
    }
}

impl Scale for (usize, ScaleRounding) {
    #[inline]
    fn get(self) -> (ScaleRounding, usize) {
        (self.1, self.0)
    }
}

impl Scale for (ScaleRounding, usize) {
    #[inline]
    fn get(self) -> (ScaleRounding, usize) {
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerialField<S: Hash + Eq> {
    /// Refences data that isn't know yet
    Dynamic {
        origin: S,
        sector: S,
        /// Index from begining of first sector
        index: usize,
        scale: usize,
        rounding: ScaleRounding,
        bytes: usize,
    },
    /// File to be loaded on build
    External {
        path: PathBuf,
        /// Is checked on build
        size: usize,
    },
    U8(u8),
    U16(u16),
    U24(u24),
    U32(u32),
    U64(u64),
    /// Variable width null terminated string
    String(String),
    Bytes(Vec<u8>),
    /// Fills data up to offset from origin
    /// Errors if past origin
    Fill {
        origin: S,
        fill: usize,
    },
}

impl<S: Hash + Eq + Clone + std::fmt::Debug> SerialField<S> {
    pub(crate) fn calculate_size(
        &self,
        offset: usize,
        tracker: &SerialTracker<S>,
    ) -> anyhow::Result<usize> {
        match self {
            // Add one for null terminator
            Self::String(value) => Ok(value.len() + 1),
            Self::Dynamic {
                sector: _,
                index: _,
                origin: _,
                scale: _,
                rounding: _,
                bytes,
            } => Ok(*bytes),
            Self::U24(_) => Ok(3),
            Self::U8(_) => Ok(1),
            Self::U16(_) => Ok(2),
            Self::U32(_) => Ok(4),
            Self::U64(_) => Ok(8),
            Self::Bytes(value) => Ok(value.len()),
            Self::External { path: _, size } => Ok(*size),
            Self::Fill { origin, fill } => {
                let origin_position = tracker.offset_from_origin(origin)?;
                Self::fill_size(offset, origin_position, *fill)
            }
        }
    }

    pub(crate) async fn build(
        &self,
        buffer: &mut (impl AsyncWrite + Unpin + AsyncSeek),
        sectors: &IndexMap<S, SerialSectorBuilder<S>>,
        tracker: &SerialTracker<S>,
    ) -> anyhow::Result<()> {
        match self {
            Self::String(value) => {
                buffer.write_all(value.as_bytes()).await?;
                buffer.write_u8(0).await?;
            }
            Self::Bytes(value) => buffer.write_all(value).await?,
            Self::Dynamic {
                sector,
                index,
                origin,
                scale,
                rounding,
                bytes,
            } => {
                let pointer =
                    tracker.offset_field_from_sector(origin, sector, *index, sectors, tracker)?;

                // Not always what the user wants
                // TODO: Add scale aligned check
                //if !pointer.is_multiple_of(*scale) {
                //    bail!(
                //        "Dynamic pointer isn't aligned to scale: {} % {} != 0, off by {}",
                //        pointer,
                //        scale,
                //        pointer % scale
                //    );
                //}

                macro_rules! match_bytes {
                    (
                        $bytes: ident,
                        $rounding: ident,
                        $pointer: ident,
                        $scale: ident,
                        [$((
                            $type: ty,
                            $byte_count: literal,
                            $try_from: ident,
                            |$p: ident| $writer: expr$(,)?
                        )),+$(,)?]$(,)?
                    ) => {
                        match $bytes {
                            $($byte_count => {
                                let $p =
                                    <$type>::$try_from($rounding.apply($pointer, *$scale) as u32).with_context(|| {
                                        format!(
                                            "Pointer exceeds {}-bit limit: {} bytes > {} bytes",
                                            <$type>::BITS,
                                            pointer,
                                            <$type>::MAX
                                        )
                                    })?;
                                $writer.await?;
                            })+,
                            _ => {
                                ::anyhow::bail!(
                                    "Unsupported dynamic pointer; length {} is unsupported",
                                    $bytes
                                )
                            }
                        }
                    };
                }

                match_bytes!(
                    bytes,
                    rounding,
                    pointer,
                    scale,
                    [
                        (u8, 1, try_from, |p| buffer.write_u8(p)),
                        (u16, 2, try_from, |p| buffer.write_u16_le(p)),
                        (u24, 3, checked_from_u32, |p| buffer
                            .write_all(&p.to_le_bytes())),
                        (u32, 4, try_from, |p| buffer.write_u32_le(p)),
                    ],
                );
            }
            Self::U8(value) => {
                buffer.write_u8(*value).await?;
            }
            Self::U16(value) => {
                buffer.write_u16(*value).await?;
            }
            Self::U24(value) => {
                buffer.write_all(&value.to_le_bytes()).await?;
            }
            Self::U32(value) => {
                buffer.write_u32(*value).await?;
            }
            Self::U64(value) => {
                buffer.write_u64(*value).await?;
            }
            Self::Fill { origin, fill } => {
                let offset = buffer.stream_position().await? as usize;
                let origin_position = tracker.offset_from_origin(origin)?;
                let fill_amount = Self::fill_size(offset, origin_position, *fill)?;
                buffer.seek(SeekFrom::Current(fill_amount as i64)).await?;
            }
            Self::External { path, size } => {
                let data = tokio::fs::read(path).await?;
                let read = buffer.write(&data).await?;

                if read != *size {
                    bail!(
                        "External file has incorrect file size:\n\
                         Expected: {size} bytes, Found: {read} bytes\n\
                         Path: {path:?}"
                    );
                }
            }
        }

        Ok(())
    }

    fn fill_size(offset: usize, origin_position: usize, fill: usize) -> anyhow::Result<usize> {
        let fill_start = offset.checked_sub(origin_position).with_context(|| format!("Failed to serialize; current position is before fill origin: {offset} < {origin_position}"))?;
        fill.checked_sub(fill_start).with_context(|| {
            format!("Failed to serialize; fill start is past fill amount: {fill_start} > {fill}")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_rounding_floor_0() {
        let rounded = ScaleRounding::Floor.apply(11, 3);

        assert_eq!(rounded, 3);
    }

    #[test]
    fn scale_rounding_floor_1() {
        let rounded = ScaleRounding::Floor.apply(10, 10);

        assert_eq!(rounded, 1);
    }

    #[test]
    fn scale_rounding_floor_2() {
        let rounded = ScaleRounding::Floor.apply(26, 5);

        assert_eq!(rounded, 5);
    }

    #[test]
    fn scale_rounding_ceiling_0() {
        let rounded = ScaleRounding::Ceiling.apply(11, 3);

        assert_eq!(rounded, 4);
    }

    #[test]
    fn scale_rounding_ceiling_1() {
        let rounded = ScaleRounding::Ceiling.apply(10, 10);

        assert_eq!(rounded, 1);
    }

    #[test]
    fn scale_rounding_ceiling_2() {
        let rounded = ScaleRounding::Ceiling.apply(26, 5);

        assert_eq!(rounded, 6);
    }

    #[test]
    fn scale_rounding_nearest_0() {
        let rounded = ScaleRounding::Nearest.apply(11, 3);

        assert_eq!(rounded, 4);
    }

    #[test]
    fn scale_rounding_nearest_1() {
        let rounded = ScaleRounding::Nearest.apply(10, 10);

        assert_eq!(rounded, 1);
    }

    #[test]
    fn scale_rounding_nearest_2() {
        let rounded = ScaleRounding::Nearest.apply(26, 5);

        assert_eq!(rounded, 5);
    }
}
