use std::{collections::HashMap, hash::Hash};

use anyhow::{Context, bail};
use indexmap::IndexMap;
use log::debug;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct SerialTracker<S: Hash + Eq> {
    sector_offsets: HashMap<S, usize>,
}

impl<S: Hash + Eq + Clone + std::fmt::Debug> SerialTracker<S> {
    pub fn offset_field_from_sector(
        &self,
        from_sector: &S,
        to_sector: &S,
        to_index: usize,
        sectors: &IndexMap<S, SerialSectorBuilder<S>>,
        tracker: &SerialTracker<S>,
    ) -> anyhow::Result<usize> {
        let from_offset = self
            .sector_offsets
            .get(from_sector)
            .cloned()
            .with_context(|| format!("Sector does not exist: {from_sector:#?}"))?;
        let to_offset = self
            .sector_offsets
            .get(to_sector)
            .cloned()
            .with_context(|| format!("Sector does not exist: {to_sector:#?}"))?;
        let mut offset = to_offset.checked_sub(from_offset).with_context(|| {
            format!("From sector was ahead of to sector: {from_offset} > {to_offset}")
        })?;

        let fields = &sectors
            .get(to_sector)
            .with_context(|| format!("Sector does not exist: {to_sector:#?}"))?
            .fields;

        if fields.len() <= to_index && to_index != 0 {
            bail!(
                "Can't index into sector; not enough fields. Sector: {:#?}, Length: {}, Index: {}",
                to_sector,
                fields.len(),
                to_index
            );
        }

        // Adds the sizes of all fields up to the index
        for (field, _) in fields.iter().zip(0..to_index) {
            offset += field.calculate_size(offset, tracker)?;
        }

        Ok(offset)
    }

    /// Caches all sector starting and ending offsets
    pub async fn new(sectors: &IndexMap<S, SerialSectorBuilder<S>>) -> anyhow::Result<Self> {
        let mut tracker = Self {
            sector_offsets: HashMap::with_capacity(sectors.len()),
        };

        let mut offset = 0;

        for (sector_id, sector) in sectors {
            let start = offset;

            for field in &sector.fields {
                offset += field.calculate_size(offset, &tracker)?;
            }

            let old_value = tracker.sector_offsets.insert(sector_id.clone(), start);

            if let Some(start) = old_value {
                bail!(
                    "Sector offsets was already populated; key: {:#?}, start: {start}",
                    sector_id
                );
            }
        }

        debug!("Tracked all sectors");

        Ok(tracker)
    }

    pub fn offset_from_origin(&self, origin_sector: &S) -> anyhow::Result<usize> {
        self.sector_offsets
            .get(origin_sector)
            .with_context(|| {
                format!("Failed to find origin; was likely in front or missing: {origin_sector:#?}")
            })
            .cloned()
    }
}
