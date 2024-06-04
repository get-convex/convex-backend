use std::{
    collections::BTreeMap,
    fs::File,
    io::{
        BufReader,
        Read,
        Write,
    },
    path::Path,
};

use byteorder::{
    LittleEndian,
    ReadBytesExt,
    WriteBytesExt,
};
use csf::ls::Map as CsfMap;

use crate::metrics::{
    load_id_tracker_timer,
    log_id_tracker_size,
};

/// Version 1 of the id table has the following format:
/// ```
/// [ version ] [ count ] [ index_len ] [ ID ]* [ index ]
/// ```
/// - version (u8): version number for the file format
/// - count (little-endian u32): number of Convex IDs in the table
/// - index_len (little-endian u32): length of `index` in bytes
/// - ID (dense array of 16 byte Convex IDs): Convex IDs in search/vector index
///   id order
/// - index: perfect hash table mapping Convex Id to search index ID.
pub const ID_TABLE_VERSION: u8 = 1;

/// Static implementation of `IdTracker` based on loading the index files
/// produced by `MemoryIdTracker`.
///
/// This implementation currently loads the full indexes into memory, but we
/// could eventually change this to use `mmap(2)` to lazily load pieces as
/// necessary.

pub struct StaticIdTracker {
    count: usize,
    /// Convex IDs in search/vector index id order.
    id_buf: Vec<u8>,
    csf_map: CsfMap,
}

impl StaticIdTracker {
    #[minitrace::trace]
    pub fn load_from_path<P: AsRef<Path>>(id_table_path: P) -> anyhow::Result<Self> {
        let _timer = load_id_tracker_timer();
        let uuid_file = File::open(id_table_path)?;
        let size = uuid_file.metadata()?.len() as usize;
        log_id_tracker_size(size);
        StaticIdTracker::load(size, BufReader::new(uuid_file))
    }

    pub fn load(file_len: usize, reader: impl Read) -> anyhow::Result<Self> {
        let (count, uuid_buf, csf_map) = Self::load_ids(file_len, reader)?;
        Ok(Self {
            count,
            id_buf: uuid_buf,
            csf_map,
        })
    }

    pub fn load_ids(
        file_len: usize,
        mut reader: impl Read,
    ) -> anyhow::Result<(usize, Vec<u8>, CsfMap)> {
        anyhow::ensure!(reader.read_u8()? == ID_TABLE_VERSION);
        let count = reader.read_u32::<LittleEndian>()? as usize;
        let index_len = reader.read_u32::<LittleEndian>()? as usize;

        // Even though the file is self-describing, check that our lengths match up to
        // be defensive against truncations.
        let mut expected_len = 0;
        expected_len += 1; // version
        expected_len += 4; // count
        expected_len += 4; // index_len
        expected_len += 16 * count; // Convex IDs
        expected_len += index_len; // index
        anyhow::ensure!(expected_len == file_len);

        let mut uuid_buf = vec![0u8; 16 * count];
        reader.read_exact(&mut uuid_buf)?;

        let csf_map = CsfMap::read(&mut reader)?;

        Ok((count, uuid_buf, csf_map))
    }

    pub fn get_convex_id(&self, search_index_id: usize) -> Option<[u8; 16]> {
        if search_index_id >= self.count {
            return None;
        }
        let buf = self.slice_from_offset(search_index_id);
        Some(buf)
    }

    /// You must verify that the offset is valid (<= count-16 at least) before
    /// calling this method or it will panic.
    fn slice_from_offset(&self, offset: usize) -> [u8; 16] {
        self.id_buf[offset * 16..(offset + 1) * 16]
            .try_into()
            .unwrap()
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn lookup(&self, convex_id: [u8; 16]) -> Option<u32> {
        let index_id = self.csf_map.get(&convex_id);
        let found_convex_id = self.get_convex_id(index_id as usize)?;
        if convex_id != found_convex_id {
            return None;
        }
        Some(index_id as u32)
    }
}

#[derive(Default, Debug)]
pub struct MemoryIdTracker {
    pub by_index_id: BTreeMap<u32, [u8; 16]>,
    pub by_convex_id: BTreeMap<[u8; 16], u32>,
}

impl MemoryIdTracker {
    pub fn insert(&mut self, index_id: u32, convex_id: [u8; 16]) {
        self.by_index_id.insert(index_id, convex_id);
        self.by_convex_id.insert(convex_id, index_id);
    }

    pub fn index_id(&self, convex_id: [u8; 16]) -> Option<u32> {
        self.by_convex_id.get(&convex_id).copied()
    }

    pub fn convex_id(&self, index_id: u32) -> Option<[u8; 16]> {
        self.by_index_id.get(&index_id).copied()
    }

    pub fn check_invariants(&mut self) -> anyhow::Result<()> {
        if let Some(last_offset) = self.by_index_id.keys().next_back() {
            anyhow::ensure!(
                *last_offset as usize == self.by_index_id.len() - 1,
                "Non-contiguous offsets"
            );
        }
        Ok(())
    }

    /// Write out a static file with the ID tracker's Convex IDs and index ids.
    pub fn write_id_tracker(&mut self, mut out: impl Write) -> anyhow::Result<()> {
        self.check_invariants()?;

        // Build up a flat array of the Convex IDs in offset order.
        let convex_ids = self.by_index_id.values().collect::<Vec<_>>();

        // Compute the number of bits needed to represent each index id.
        let index_id_bits = convex_ids.len().next_power_of_two().trailing_zeros();

        // Build the perfect hash table.
        let map =
            CsfMap::try_with_fn::<_, _, ()>(&convex_ids, |i, _| i as u64, index_id_bits as u8)
                .ok_or_else(|| anyhow::anyhow!("Failed to create CsfMap"))?;

        out.write_u8(ID_TABLE_VERSION)?;
        out.write_u32::<LittleEndian>(convex_ids.len().try_into()?)?;
        out.write_u32::<LittleEndian>(map.write_bytes().try_into()?)?;
        for convex_id in &convex_ids {
            out.write_all(&convex_id[..])?;
        }
        map.write(&mut out)?;

        out.flush()?;

        Ok(())
    }
}
