use std::{hash::Hash, io::SeekFrom, path::PathBuf};

use anyhow::{Context, bail};
use indexmap::IndexMap;
use tokio::io::{AsyncSeek, AsyncSeekExt, AsyncWrite, AsyncWriteExt};
use u24::u24;

use crate::{prelude::*, tracker::SerialTracker};

#[derive(Debug, Clone)]
pub enum SerialField<S: Hash + Eq> {
    /// Refences data that isn't know yet
    Dynamic {
        origin: S,
        sector: S,
        /// Index from begining of first sector
        index: usize,
        scale: usize,
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
            }
            | Self::U24(_) => Ok(3),
            Self::U8(_) => Ok(1),
            Self::U16(_) => Ok(2),
            Self::U32(_) => Ok(4),
            Self::U64(_) => Ok(8),
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
            Self::Dynamic {
                sector,
                index,
                origin,
                scale,
            } => {
                let pointer =
                    tracker.offset_field_from_sector(origin, sector, *index, sectors, tracker)?;

                if !pointer.is_multiple_of(*scale) {
                    bail!(
                        "Dynamic pointer isn't aligned to scale: {} % {} != 0, off by {}",
                        pointer,
                        scale,
                        pointer % scale
                    );
                }

                let pointer =
                    u24::checked_from_u32((pointer / scale) as u32).with_context(|| {
                        format!(
                            "Pointer exceeds 24-bit limit: {} bytes > {} bytes",
                            pointer,
                            u24::MAX
                        )
                    })?;
                buffer.write_all(&pointer.to_le_bytes()).await?;
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
