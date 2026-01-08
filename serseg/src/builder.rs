use std::{hash::Hash, path::PathBuf};

use indexmap::IndexMap;
use log::debug;
use tokio::io::{AsyncSeek, AsyncWrite, AsyncWriteExt};
use u24::u24;

use crate::{
    field::{Scale, ScaleRounding, SerialField},
    tracker::SerialTracker,
};

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

macro_rules! int_field {
    ($field_name: ident, $unsigned: ident) => {
        pub fn $unsigned(self, value: impl Into<$unsigned>) -> Self {
            self.field(SerialField::$field_name(value.into()))
        }
    };
    ($field_name: ident, $unsigned: ident, $signed: ident) => {
        int_field!($field_name, $unsigned);

        pub fn $signed(self, value: impl Into<$signed>) -> Self {
            self.field(SerialField::$field_name(value.into() as $unsigned))
        }
    };
}

macro_rules! null_field {
    ($size: literal) => {
        pub fn ${concat(null_, $size)}(self) -> Self {
            self.field(SerialField::${concat(U, $size)}(::std::default::Default::default()))
        }
    };
}

macro_rules! dynamic_field {
    ($name: ident, $bytes: literal) => {
        pub fn ${concat(dynamic_, $name)}(self, origin: S, sector: S, index: usize) -> Self {
            self.field(SerialField::Dynamic {
                origin,
                sector,
                index,
                rounding: ScaleRounding::default(),
                scale: 1,
                bytes: $bytes,
            })
        }

        pub fn ${concat(dynamic_, $name, _chunk)}(
            self,
            origin: S,
            sector: S,
            index: usize,
            scale: impl Scale,
        ) -> Self {
            let (rounding, scale) = scale.get();

            self.field(SerialField::Dynamic {
                origin,
                sector,
                index,
                rounding,
                scale,
                bytes: $bytes,
            })
        }
    };
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

    int_field!(U8, u8, i8);
    int_field!(U16, u16, i16);
    int_field!(U24, u24);
    int_field!(U32, u32, i32);
    int_field!(U64, u64, i64);

    null_field!(8);
    null_field!(16);
    null_field!(24);
    null_field!(32);
    null_field!(64);

    dynamic_field!(u8, 1);
    dynamic_field!(u16, 2);
    dynamic_field!(u24, 3);
    dynamic_field!(u32, 4);

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
