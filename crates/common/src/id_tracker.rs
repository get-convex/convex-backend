use std::{
    fs::File,
    io::{
        BufReader,
        Read,
    },
    path::PathBuf,
};

use byteorder::{
    LittleEndian,
    ReadBytesExt,
};
use csf::ls::Map as CsfMap;

use crate::deleted_bitset::DeletedBitset;

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
    deleted: DeletedBitset,
}

impl StaticIdTracker {
    pub fn load_from_path(
        id_table_path: PathBuf,
        deleted_bitset: DeletedBitset,
    ) -> anyhow::Result<Self> {
        let uuid_file = File::open(id_table_path)?;
        StaticIdTracker::load(
            (
                uuid_file.metadata()?.len() as usize,
                BufReader::new(uuid_file),
            ),
            deleted_bitset,
        )
    }

    pub fn load(
        uuid_file: (usize, impl Read),
        deleted_bitset: DeletedBitset,
    ) -> anyhow::Result<Self> {
        let (count, uuid_buf, csf_map) = Self::load_ids(uuid_file.0, uuid_file.1)?;
        anyhow::ensure!(count == deleted_bitset.len());
        Ok(Self {
            count,
            id_buf: uuid_buf,
            csf_map,
            deleted: deleted_bitset,
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

    pub fn deleted(&self) -> &DeletedBitset {
        &self.deleted
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
