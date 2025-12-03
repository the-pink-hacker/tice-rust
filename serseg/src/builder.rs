use std::{hash::Hash, path::PathBuf};

use indexmap::IndexMap;
use log::debug;
use tokio::io::{AsyncSeek, AsyncWrite, AsyncWriteExt};
use u24::u24;

use crate::{field::SerialField, tracker::SerialTracker};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialBuilder<S: Hash + Eq + Clone + std::fmt::Debug> {
    sectors: IndexMap<S, SerialSectorBuilder<S>>,
}

// Default macro requires S to implement default
// We don't want that
impl<S: Hash + Eq + Clone + std::fmt::Debug> Default for SerialBuilder<S> {
    fn default() -> Self {
        Self {
            sectors: IndexMap::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialSectorBuilder<S: Hash + Eq> {
    pub(crate) fields: Vec<SerialField<S>>,
}

// Default macro requires S to implement default
// We don't want that
impl<S: Hash + Eq + std::fmt::Debug> Default for SerialSectorBuilder<S> {
    fn default() -> Self {
        Self {
            fields: Vec::default(),
        }
    }
}

impl<S: Hash + Eq + Clone + std::fmt::Debug> SerialBuilder<S> {
    pub fn sector(mut self, key: S, builder: SerialSectorBuilder<S>) -> Self {
        self.sectors.insert(key, builder);
        self
    }

    pub fn sector_default(self, key: S) -> Self {
        self.sector(key, SerialSectorBuilder::<S>::default())
    }

    pub async fn build(
        self,
        buffer: &mut (impl AsyncWrite + Unpin + AsyncSeek),
    ) -> anyhow::Result<()> {
        let tracker = SerialTracker::new(&self.sectors).await?;

        for (sector_id, sector) in &self.sectors {
            sector.build(buffer, &self.sectors, &tracker).await?;
            debug!("Built sector: {sector_id:#?}");
        }

        buffer.flush().await?;

        Ok(())
    }
}

impl<S: Hash + Eq + Clone + std::fmt::Debug> SerialSectorBuilder<S> {
    fn field(mut self, field: SerialField<S>) -> Self {
        self.fields.push(field);
        self
    }

    pub fn string(self, value: impl Into<String>) -> Self {
        self.field(SerialField::String(value.into()))
    }

    pub fn bytes(self, value: impl IntoIterator<Item = u8>) -> Self {
        self.field(SerialField::Bytes(value.into_iter().collect()))
    }

    pub fn u8(self, value: impl Into<u8>) -> Self {
        self.field(SerialField::U8(value.into()))
    }

    pub fn i8(self, value: impl Into<i8>) -> Self {
        self.field(SerialField::U8(value.into() as u8))
    }

    pub fn u16(self, value: impl Into<u16>) -> Self {
        self.field(SerialField::U16(value.into()))
    }

    pub fn i16(self, value: impl Into<i16>) -> Self {
        self.field(SerialField::U16(value.into() as u16))
    }

    pub fn u24(self, value: impl Into<u24>) -> Self {
        self.field(SerialField::U24(value.into()))
    }

    pub fn u32(self, value: impl Into<u32>) -> Self {
        self.field(SerialField::U32(value.into()))
    }

    pub fn i32(self, value: impl Into<i32>) -> Self {
        self.field(SerialField::U32(value.into() as u32))
    }

    pub fn u64(self, value: impl Into<u64>) -> Self {
        self.field(SerialField::U64(value.into()))
    }

    pub fn i64(self, value: impl Into<i64>) -> Self {
        self.field(SerialField::U64(value.into() as u64))
    }

    pub fn null_16(self) -> Self {
        self.field(SerialField::U16(0))
    }

    pub fn null_24(self) -> Self {
        self.field(SerialField::U24(u24::from_le_bytes([0, 0, 0])))
    }

    pub fn dynamic_u16(self, origin: S, sector: S, index: usize) -> Self {
        self.field(SerialField::Dynamic {
            origin,
            sector,
            index,
            scale: 1,
            bytes: 2,
        })
    }

    pub fn dynamic_u16_chunk(self, origin: S, sector: S, index: usize, scale: usize) -> Self {
        self.field(SerialField::Dynamic {
            origin,
            sector,
            index,
            scale,
            bytes: 2,
        })
    }

    pub fn dynamic_u24(self, origin: S, sector: S, index: usize) -> Self {
        self.field(SerialField::Dynamic {
            origin,
            sector,
            index,
            scale: 1,
            bytes: 3,
        })
    }

    pub fn dynamic_u24_chunk(self, origin: S, sector: S, index: usize, scale: usize) -> Self {
        self.field(SerialField::Dynamic {
            origin,
            sector,
            index,
            scale,
            bytes: 3,
        })
    }

    pub fn fill(self, origin: S, fill: usize) -> Self {
        self.field(SerialField::Fill { origin, fill })
    }

    pub fn external(self, path: impl Into<PathBuf>, size: usize) -> Self {
        self.field(SerialField::External {
            path: path.into(),
            size,
        })
    }

    async fn build(
        &self,
        buffer: &mut (impl AsyncWrite + Unpin + AsyncSeek),
        sectors: &IndexMap<S, SerialSectorBuilder<S>>,
        tracker: &SerialTracker<S>,
    ) -> anyhow::Result<()> {
        for field in &self.fields {
            field.build(buffer, sectors, tracker).await?;
        }

        Ok(())
    }
}
