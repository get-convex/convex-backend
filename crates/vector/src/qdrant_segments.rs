use std::{
    collections::HashMap,
    fs::{
        self,
        File,
    },
    io::BufWriter,
    path::{
        Path,
        PathBuf,
    },
    sync::{
        atomic::AtomicBool,
        Arc,
    },
};

use atomic_refcell::AtomicRefCell;
use common::{
    deleted_bitset::DeletedBitset,
    id_tracker::StaticIdTracker,
    runtime::tokio_spawn_blocking,
};
use parking_lot::{
    Mutex,
    RwLock,
};
use qdrant_common::cpu::CpuPermit;
use qdrant_segment::vector_storage::{
    appendable_mmap_dense_vector_storage::open_appendable_memmap_vector_storage,
    memmap_dense_vector_storage::open_memmap_vector_storage,
};
use qdrant_segment::{
    common::{
        rocksdb_wrapper::{
            db_options,
            // open_db,
            DB_MAPPING_CF,
            DB_PAYLOAD_CF,
            DB_VECTOR_CF,
            DB_VERSIONS_CF,
        },
        version::StorageVersion,
    },
    entry::entry_point::SegmentEntry,
    id_tracker::IdTracker,
    index::{
        hnsw_index::{
            graph_links::GraphLinksMmap,
            hnsw::HNSWIndex,
        },
        plain_payload_index::PlainIndex,
        struct_payload_index::StructPayloadIndex,
        VectorIndexEnum,
    },
    payload_storage::on_disk_payload_storage::OnDiskPayloadStorage,
    segment::{
        Segment,
        SegmentVersion,
        VectorData,
    },
    segment_constructor::{
        get_vector_index_path,
        get_vector_storage_path,
        segment_builder::SegmentBuilder,
        PAYLOAD_INDEX_PATH,
    },
    types::{
        Distance,
        HnswConfig,
        Indexes,
        PayloadStorageType,
        SegmentConfig,
        SegmentType,
        VectorDataConfig,
        VectorStorageType,
        DEFAULT_FULL_SCAN_THRESHOLD,
        DEFAULT_HNSW_EF_CONSTRUCT,
    },
    vector_storage::VectorStorage,
};
use rocksdb::DB;

use crate::id_tracker::{
    VectorMemoryIdTracker,
    VectorStaticIdTracker,
};

const UUID_TABLE_FILENAME: &str = "uuids.table";
const DELETED_BITSET_FILENAME: &str = "deleted.bitset";
pub(crate) const DEFAULT_VECTOR_NAME: &str = "default_vector";

pub(crate) fn segment_config(
    dimension: usize,
    mutable: bool,
    max_indexing_threads: usize,
) -> SegmentConfig {
    let index = if mutable {
        Indexes::Plain {}
    } else {
        let hnsw_config = HnswConfig {
            // Number of edges per node in the index graph. Larger the value -
            // more accurate the search, more space required.
            m: 16,
            // Number of neighbours to consider during the index building.
            // Larger  the value - more accurate the search, more
            // time required to build index.
            ef_construct: DEFAULT_HNSW_EF_CONSTRUCT,
            // Minimal size (in KiloBytes) of vectors for additional
            // payload-based indexing. If payload chunk is smaller
            // than `full_scan_threshold_kb` additional indexing
            // won't be used - in this case full-scan search should be
            // preferred by query planner and additional indexing
            // is not required. Note: 1Kb = 1 vector of size 256
            full_scan_threshold: DEFAULT_FULL_SCAN_THRESHOLD,
            max_indexing_threads,
            on_disk: Some(true),
            // Custom M param for hnsw graph built for payload index. If not
            // set, default M will be used.
            payload_m: None,
        };
        Indexes::Hnsw(hnsw_config)
    };
    let vector_storage_type = if mutable {
        VectorStorageType::ChunkedMmap
    } else {
        VectorStorageType::Mmap
    };
    let vector_data_config = VectorDataConfig {
        size: dimension,
        distance: Distance::Cosine,
        storage_type: vector_storage_type,
        index,
        quantization_config: None,
    };
    SegmentConfig {
        vector_data: HashMap::from([(DEFAULT_VECTOR_NAME.to_string(), vector_data_config)]),
        sparse_vector_data: Default::default(),
        payload_storage_type: PayloadStorageType::OnDisk,
    }
}

pub fn create_mutable_segment(
    path: &Path,
    id_tracker: Arc<AtomicRefCell<VectorMemoryIdTracker>>,
    dimension: usize,
    segment_config: SegmentConfig,
) -> anyhow::Result<Segment> {
    fs::create_dir_all(path)?;

    let vector_db_names = vec![format!("{DB_VECTOR_CF}-{DEFAULT_VECTOR_NAME}")];
    let database = open_db(path, &vector_db_names, false)?;
    let payload_storage = OnDiskPayloadStorage::open(database.clone())?;

    let payload_index_path = path.join(PAYLOAD_INDEX_PATH);
    let payload_index = StructPayloadIndex::open(
        Arc::new(AtomicRefCell::new(payload_storage.into())),
        id_tracker.clone(),
        &payload_index_path,
        true,
    )?;
    let payload_index = Arc::new(AtomicRefCell::new(payload_index));

    let stopped = AtomicBool::new(false);
    let vector_storage_path = get_vector_storage_path(path, DEFAULT_VECTOR_NAME);
    let vector_storage = open_appendable_memmap_vector_storage(
        &vector_storage_path,
        dimension,
        Distance::Cosine,
        &stopped,
    )?;
    let point_count = id_tracker.borrow().total_point_count();
    let vector_count = vector_storage.borrow().total_vector_count();
    anyhow::ensure!(point_count == vector_count);

    let vector_index = VectorIndexEnum::Plain(PlainIndex::new(
        id_tracker.clone(),
        vector_storage.clone(),
        payload_index.clone(),
    ));
    let vector_index = Arc::new(AtomicRefCell::new(vector_index));
    let vector_data = VectorData {
        vector_storage,
        vector_index,
        quantized_vectors: Arc::new(Default::default()),
    };
    let segment = Segment {
        version: None,
        persisted_version: Arc::new(Mutex::new(None)),
        current_path: path.to_owned(),
        id_tracker,
        vector_data: HashMap::from([(DEFAULT_VECTOR_NAME.to_string(), vector_data)]),
        segment_type: SegmentType::Plain,
        appendable_flag: true,
        payload_index,
        segment_config,
        error_status: None,
        database,
        flush_thread: Mutex::new(None),
    };
    segment.save_current_state()?;
    SegmentVersion::save(path)?;
    Ok(segment)
}

/// A set of paths for a fragmented vector segment where `segment` points to a
/// single tar file generated by snapshotting a qdrant segment.
#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub struct VectorDiskSegmentPaths {
    pub segment: PathBuf,
    pub uuids: PathBuf,
    pub deleted_bitset: PathBuf,
}

/// A set of paths for a fragmented vector segment where `segment_dir` points to
/// a directory into which we've unpacked a tar file generated from snapshotting
/// a qdrant segment.
#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub struct UntarredVectorDiskSegmentPaths {
    segment_dir: PathBuf,
    uuids: PathBuf,
    deleted_bitset: PathBuf,
}

impl UntarredVectorDiskSegmentPaths {
    pub fn from(untarred_segment: PathBuf, paths: VectorDiskSegmentPaths) -> Self {
        Self::new(untarred_segment, paths.uuids, paths.deleted_bitset)
    }

    // We could verify that untarred_segment is a directory here, but we don't
    // want to do blocking IO on tokio's threads and we can't rely on tokio
    // being present to safely do non-blocking IO.
    pub fn new(untarred_segment: PathBuf, uuids: PathBuf, deleted_bitset: PathBuf) -> Self {
        Self {
            segment_dir: untarred_segment,
            uuids,
            deleted_bitset,
        }
    }
}

#[derive(Debug)]
pub struct VectorDiskSegmentValues {
    pub paths: VectorDiskSegmentPaths,
    pub num_vectors: u32,
    pub num_deleted: u32,
}

pub fn build_disk_segment(
    segment: &Segment,
    tmp_path: &Path,
    disk_path: &Path,
    segment_config: SegmentConfig,
) -> anyhow::Result<VectorDiskSegmentValues> {
    merge_disk_segments(vec![(None, segment)], tmp_path, disk_path, segment_config)
}

pub fn merge_disk_segments_hnsw(
    segments: Vec<(Option<UntarredVectorDiskSegmentPaths>, &Segment)>,
    dimension: usize,
    tmp_path: &Path,
    disk_path: &Path,
) -> anyhow::Result<VectorDiskSegmentValues> {
    let segment_config = segment_config(dimension, false, 4);
    merge_disk_segments(segments, tmp_path, disk_path, segment_config)
}

pub fn merge_disk_segments(
    segments: Vec<(Option<UntarredVectorDiskSegmentPaths>, &Segment)>,
    tmp_path: &Path,
    disk_path: &Path,
    segment_config: SegmentConfig,
) -> anyhow::Result<VectorDiskSegmentValues> {
    // Create separate tmp paths to avoid unexpected overlap with the original
    // segment or various intermediate outputs. Because qdrant restores segments
    // to paths adjacent to the path where we save the snapshot, we want to
    // ensure that the directory where we create the snapshot (disk_path) is
    // clean.
    let tmp_segment_path = tmp_path.join("merged_segment");
    std::fs::create_dir(tmp_segment_path.clone())?;
    let segment_tmp_dir_path = tmp_path.join("merged_segment_tmp");
    std::fs::create_dir(segment_tmp_dir_path.clone())?;
    let snapshot_tmp_path = tmp_path.join("snapshot_tmp");
    std::fs::create_dir(snapshot_tmp_path.clone())?;

    let mut segment_builder =
        SegmentBuilder::new(&tmp_segment_path, &segment_tmp_dir_path, &segment_config)?;
    let stopped = AtomicBool::new(false);
    for (paths, segment) in segments {
        tracing::info!("Updating new segment with segment from paths {:?}", paths);
        segment_builder.update_from(segment, &stopped)?;
        if let Some(ref segment) = segment_builder.segment {
            anyhow::ensure!(segment.id_tracker.borrow().deleted_point_count() == 0);
        }
    }
    let permit = CpuPermit::dummy(4);
    let disk_segment = segment_builder.build(permit, &stopped)?;

    // The disk segment we just built was using a qdrant id tracker. We now need to
    // construct our own id tracker with the same set of ids. We could do this
    // by making SegmentBuilder use our id tracker if this turns out to be a
    // performance issue. However doing so requires a lot of additional code to
    // duplicate the logic SegmentBuilder is using to create the segment.
    // Ideally the performance penalty here is small relative to the overall
    // build cost and is worth the simpler code.
    let mut memory_tracker = VectorMemoryIdTracker::new();
    let borrowed_tracker = disk_segment.id_tracker.borrow();
    // All deletes should be hard deletes. Deleted vectors should be excluded
    // entirely, not marked as soft deleted.
    anyhow::ensure!(borrowed_tracker.deleted_point_count() == 0);

    for internal_id in borrowed_tracker.iter_internal() {
        let external_id = borrowed_tracker
            .external_id(internal_id)
            .expect("Missing external id!");
        memory_tracker.set_link(external_id, internal_id)?;
    }

    // Sanity check that we included all of the points.
    anyhow::ensure!(memory_tracker.total_point_count() == borrowed_tracker.total_point_count());

    // Sanity check that we didn't insert multile vectors for any given id.
    let total_point_count = memory_tracker.total_point_count();
    let vector_data = disk_segment.vector_data.get(DEFAULT_VECTOR_NAME).unwrap();
    let vector_count = vector_data.vector_storage.borrow().total_vector_count();
    let num_deleted = memory_tracker.deleted_point_count();
    anyhow::ensure!(vector_count == total_point_count);
    // Writing the new segment should have removed all deletes.
    anyhow::ensure!(num_deleted == 0);

    let memory_tracker = Arc::new(AtomicRefCell::new(memory_tracker));
    let segment = snapshot_segment(
        &memory_tracker,
        &disk_segment,
        &snapshot_tmp_path,
        disk_path,
    )?;
    Ok(VectorDiskSegmentValues {
        paths: segment,
        num_vectors: total_point_count as u32,
        num_deleted: num_deleted as u32,
    })
}

pub fn snapshot_segment(
    id_tracker: &Arc<AtomicRefCell<VectorMemoryIdTracker>>,
    segment: &Segment,
    tmp_path: &Path,
    index_path: &Path,
) -> anyhow::Result<VectorDiskSegmentPaths> {
    let segment_path = segment.take_snapshot(tmp_path, index_path)?;

    // Write out our additional index files for the ID tracker.
    let mut id_tracker = id_tracker.borrow_mut();
    let uuids_path = index_path.join(UUID_TABLE_FILENAME);
    {
        let mut out = BufWriter::new(File::create(uuids_path.clone())?);
        id_tracker.write_uuids(&mut out)?;
        out.into_inner()?.sync_all()?;
    }
    let deleted_bitset_path = index_path.join(DELETED_BITSET_FILENAME);
    {
        let mut out = BufWriter::new(File::create(deleted_bitset_path.clone())?);
        id_tracker.write_deleted_bitset(&mut out)?;
        out.into_inner()?.sync_all()?;
    }
    Ok(VectorDiskSegmentPaths {
        segment: segment_path,
        uuids: uuids_path,
        deleted_bitset: deleted_bitset_path,
    })
}

pub async fn restore_segment_from_tar(archive_path: &Path) -> anyhow::Result<PathBuf> {
    // This is taken directly from Qdrant's tests...
    let segment_id = archive_path
        .file_stem()
        .and_then(|f| f.to_str())
        .unwrap()
        .to_owned();

    // As is this...
    let out_path = archive_path
        .parent()
        .expect("Failed to obtain parent for archive")
        .join(&segment_id);

    let archive_path = archive_path.to_owned();
    tokio_spawn_blocking("segment_restore_snapshot", move || {
        Segment::restore_snapshot(&archive_path, &segment_id)
    })
    .await??;

    Ok(out_path)
}

fn open_db<T: AsRef<str>>(
    path: &Path,
    vector_pathes: &[T],
    read_only: bool,
) -> Result<Arc<RwLock<DB>>, rocksdb::Error> {
    let mut column_families = vec![DB_PAYLOAD_CF, DB_MAPPING_CF, DB_VERSIONS_CF];
    for vector_path in vector_pathes {
        column_families.push(vector_path.as_ref());
    }

    let db = if read_only {
        DB::open_cf_for_read_only(&db_options(), path, column_families, false)?
    } else {
        DB::open_cf(&db_options(), path, column_families)?
    };

    Ok(Arc::new(RwLock::new(db)))
}

/// Loads a qdrant Segment from a set of paths
///
/// This method is unsafe because it cannot be called concurrently or in
/// sequence for a given set of paths while the Segment is open. Loading the
/// segment involves copying the files in the Segment to a fixed path adjacent
/// to the provided path. That copy is non-atomic and concurrent copies can
/// squash or remove files used by other copies.
///
/// Unless you can be sure that the paths you provide are temporary and will be
/// used exactly once per Segment open, use FragmentedSegmentLoader instead of
/// this method.
#[cfg(any(test, feature = "testing"))]
pub async fn unsafe_load_disk_segment(paths: &VectorDiskSegmentPaths) -> anyhow::Result<Segment> {
    let path = restore_segment_from_tar(&paths.segment).await?;
    let paths = UntarredVectorDiskSegmentPaths::from(path, paths.clone());
    load_disk_segment(paths)
}

pub fn load_disk_segment(paths: UntarredVectorDiskSegmentPaths) -> anyhow::Result<Segment> {
    let untarred_path = paths.segment_dir;
    if !SegmentVersion::check_exists(&untarred_path) {
        anyhow::bail!("Missing segment version for {untarred_path:?}");
    }
    let stored_version = SegmentVersion::load(&untarred_path)?;
    let app_version = SegmentVersion::current();
    anyhow::ensure!(stored_version == app_version);

    let segment_state = Segment::load_state(&untarred_path)?;
    let vector_config = &segment_state.config.vector_data[DEFAULT_VECTOR_NAME];
    let segment_config = &segment_state.config;
    let vector_db_names = vec![format!("{DB_VECTOR_CF}-{DEFAULT_VECTOR_NAME}")];
    let database = open_db(&untarred_path, &vector_db_names, true)?;

    let payload_storage = OnDiskPayloadStorage::open(database.clone())?;

    let deleted_bitset = DeletedBitset::load_from_path(paths.deleted_bitset)?;
    let id_tracker = VectorStaticIdTracker {
        id_tracker: StaticIdTracker::load_from_path(paths.uuids)?,
        deleted_bitset,
    };
    let id_tracker = Arc::new(AtomicRefCell::new(id_tracker));

    let payload_index_path = untarred_path.join(PAYLOAD_INDEX_PATH);

    let payload_index = StructPayloadIndex::open_read_only(
        Arc::new(AtomicRefCell::new(payload_storage.into())),
        id_tracker.clone(),
        &payload_index_path,
    )?;
    let payload_index = Arc::new(AtomicRefCell::new(payload_index));

    let vector_storage_path = get_vector_storage_path(&untarred_path, DEFAULT_VECTOR_NAME);
    let vector_index_path = get_vector_index_path(&untarred_path, DEFAULT_VECTOR_NAME);
    let vector_storage = match vector_config.storage_type {
        VectorStorageType::Memory => anyhow::bail!("VectorStorageType::Memory is unsupported"),
        VectorStorageType::Mmap => open_memmap_vector_storage(
            &vector_storage_path,
            vector_config.size,
            vector_config.distance,
        )?,
        VectorStorageType::ChunkedMmap => {
            let stopped = AtomicBool::new(false);
            open_appendable_memmap_vector_storage(
                &vector_storage_path,
                vector_config.size,
                vector_config.distance,
                &stopped,
            )?
        },
    };
    let point_count = id_tracker.borrow().total_point_count();
    let vector_count = vector_storage.borrow().total_vector_count();
    anyhow::ensure!(vector_count == point_count);

    let vector_index = match vector_config.index {
        qdrant_segment::types::Indexes::Plain {} => VectorIndexEnum::Plain(PlainIndex::new(
            id_tracker.clone(),
            vector_storage.clone(),
            payload_index.clone(),
        )),
        qdrant_segment::types::Indexes::Hnsw(ref hnsw_config) => {
            anyhow::ensure!(
                hnsw_config.on_disk.unwrap_or(false),
                "HNSW indexes are only supported when they're written to disk!"
            );
            VectorIndexEnum::HnswMmap(HNSWIndex::<GraphLinksMmap>::open(
                &vector_index_path,
                id_tracker.clone(),
                vector_storage.clone(),
                Arc::new(AtomicRefCell::new(None)),
                payload_index.clone(),
                hnsw_config.clone(),
            )?)
        },
    };
    let vector_index = Arc::new(AtomicRefCell::new(vector_index));

    let vector_data = VectorData {
        vector_storage,
        vector_index,
        quantized_vectors: Arc::new(AtomicRefCell::new(None)),
    };
    let segment = Segment {
        version: segment_state.version,
        persisted_version: Arc::new(Mutex::new(segment_state.version)),
        current_path: untarred_path,
        id_tracker,
        vector_data: HashMap::from([(DEFAULT_VECTOR_NAME.to_string(), vector_data)]),
        segment_type: if segment_config.is_any_vector_indexed() {
            SegmentType::Indexed
        } else {
            SegmentType::Plain
        },
        appendable_flag: false,
        payload_index,
        segment_config: segment_config.clone(),
        error_status: None,
        database,
        flush_thread: Mutex::new(None),
    };
    Ok(segment)
}

pub trait SegmentConfigExt {
    fn dimensions(&self) -> usize;
}

impl SegmentConfigExt for SegmentConfig {
    fn dimensions(&self) -> usize {
        self.vector_data[DEFAULT_VECTOR_NAME].size
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        fs::File,
        io::BufWriter,
        sync::{
            atomic::AtomicBool,
            Arc,
        },
    };

    use anyhow::Context;
    use atomic_refcell::AtomicRefCell;
    use common::{
        deleted_bitset::DeletedBitset,
        id_tracker::StaticIdTracker,
    };
    use futures::try_join;
    use must_let::must_let;
    use qdrant_segment::{
        data_types::{
            named_vectors::NamedVectors,
            vectors::{
                QueryVector,
                Vector,
            },
        },
        entry::entry_point::SegmentEntry,
        id_tracker::IdTracker,
        json_path::JsonPath,
        segment::Segment,
        types::{
            Condition,
            ExtendedPointId,
            FieldCondition,
            Filter,
            Match,
            MatchValue,
            PayloadFieldSchema,
            PayloadSchemaType,
            PayloadSelector,
            PayloadSelectorInclude,
            PointIdType,
            SegmentConfig,
            ValueVariants,
            WithPayload,
            WithVector,
        },
    };
    use rand::{
        rngs::ThreadRng,
        Rng,
    };
    use serde_json::Value as JsonValue;
    use tempfile::TempDir;
    use uuid::Uuid;
    use value::base64;

    use crate::{
        id_tracker::{
            VectorMemoryIdTracker,
            VectorStaticIdTracker,
            OP_NUM,
        },
        qdrant_segments::{
            build_disk_segment,
            create_mutable_segment,
            merge_disk_segments,
            segment_config,
            snapshot_segment,
            unsafe_load_disk_segment,
            VectorDiskSegmentPaths,
            VectorDiskSegmentValues,
            DEFAULT_VECTOR_NAME,
        },
    };

    const DIMENSIONS: usize = 1536;
    const TEST_PAYLOAD_PATH: &str = "some_path";

    fn stream_vectors_with_test_payload(
        count: usize,
    ) -> impl Iterator<Item = (ExtendedPointId, Vec<f32>, JsonValue)> {
        stream_vectors(count)
            .map(|(point_id, vector)| (point_id, vector, create_test_payload(point_id)))
    }

    fn stream_vectors(count: usize) -> impl Iterator<Item = (ExtendedPointId, Vec<f32>)> {
        let mut rng = rand::rng();
        (0u128..(count as u128)).map(move |_| {
            let uuid = Uuid::new_v4();
            let point_id = PointIdType::Uuid(uuid);
            let v = random_vector(&mut rng, DIMENSIONS);
            (point_id, v)
        })
    }

    fn random_vector(rng: &mut ThreadRng, dimensions: usize) -> Vec<f32> {
        (0..dimensions)
            .map(|_| rng.random::<f32>())
            .collect::<Vec<_>>()
    }

    fn create_test_memory_segment(
        dimensions: usize,
        test_dir: &TempDir,
        vectors: impl Iterator<Item = (ExtendedPointId, Vec<f32>)>,
    ) -> anyhow::Result<(Segment, Arc<AtomicRefCell<VectorMemoryIdTracker>>)> {
        let memory_path = test_dir.path().join("memory");
        let id_tracker = Arc::new(AtomicRefCell::new(VectorMemoryIdTracker::new()));
        let mutable_config = segment_config(dimensions, true, 4);
        let mut memory_segment =
            create_mutable_segment(&memory_path, id_tracker.clone(), dimensions, mutable_config)?;

        for (point_id, v) in vectors {
            let vector = Vector::Dense(v);
            let named_vector = NamedVectors::from_ref(DEFAULT_VECTOR_NAME, vector.to_vec_ref());
            memory_segment.upsert_point(OP_NUM, point_id, named_vector)?;
        }

        Ok((memory_segment, id_tracker))
    }

    fn create_test_memory_segment_with_payload(
        dimensions: usize,
        test_dir: &TempDir,
        vectors: impl Iterator<Item = (ExtendedPointId, Vec<f32>, JsonValue)>,
    ) -> anyhow::Result<(Segment, Arc<AtomicRefCell<VectorMemoryIdTracker>>)> {
        let memory_path = test_dir.path().join("memory");
        let id_tracker = Arc::new(AtomicRefCell::new(VectorMemoryIdTracker::new()));
        let mutable_config = segment_config(dimensions, true, 4);
        let mut memory_segment =
            create_mutable_segment(&memory_path, id_tracker.clone(), dimensions, mutable_config)?;

        for (point_id, v, payload) in vectors {
            let vector = Vector::Dense(v);
            let named_vector = NamedVectors::from_ref(DEFAULT_VECTOR_NAME, vector.to_vec_ref());
            memory_segment.upsert_point(OP_NUM, point_id, named_vector)?;
            memory_segment.set_payload(OP_NUM, point_id, &payload.into(), &None)?;
        }

        Ok((memory_segment, id_tracker))
    }

    fn create_test_disk_segment(
        dimensions: usize,
        test_dir: &TempDir,
        vectors: impl Iterator<Item = (ExtendedPointId, Vec<f32>)>,
    ) -> anyhow::Result<VectorDiskSegmentPaths> {
        // Generate the memory segment
        let (memory_segment, _) = create_test_memory_segment(dimensions, test_dir, vectors)?;

        // Build the disk segment
        let indexing_path = test_dir.path().join("indexing");
        fs::create_dir_all(&indexing_path)?;
        let disk_path = test_dir.path().join("disk");
        fs::create_dir_all(&disk_path)?;

        let disk_config = segment_config(dimensions, false, 4);
        Ok(build_disk_segment(&memory_segment, &indexing_path, &disk_path, disk_config)?.paths)
    }

    fn include_test_payload() -> WithPayload {
        let payload_selector = PayloadSelectorInclude {
            include: vec![JsonPath::try_from(TEST_PAYLOAD_PATH).unwrap()],
        };
        WithPayload {
            enable: true,
            payload_selector: Some(PayloadSelector::Include(payload_selector)),
        }
    }

    fn create_test_payload(point_id: ExtendedPointId) -> JsonValue {
        must_let!(let ExtendedPointId::Uuid(uuid) = point_id);
        let mut map = serde_json::Map::new();
        map.insert(
            TEST_PAYLOAD_PATH.to_string(),
            JsonValue::String(base64::encode_urlsafe(&uuid.to_bytes_le())),
        );
        map.into()
    }

    fn create_test_payload_index(segment: &mut Segment) -> anyhow::Result<()> {
        let field_schema = Some(&PayloadFieldSchema::FieldType(PayloadSchemaType::Keyword));
        segment.create_field_index(
            OP_NUM,
            &JsonPath::try_from(TEST_PAYLOAD_PATH).unwrap(),
            field_schema,
        )?;
        Ok(())
    }

    fn test_payload_should_equal(point_id: ExtendedPointId) -> Filter {
        must_let!(let ExtendedPointId::Uuid(uuid) = point_id);
        let qdrant_match = Match::Value(MatchValue {
            value: ValueVariants::Keyword(base64::encode_urlsafe(&uuid.to_bytes_le())),
        });
        let conditions = vec![Condition::Field(FieldCondition::new_match(
            JsonPath::try_from(TEST_PAYLOAD_PATH).unwrap(),
            qdrant_match,
        ))];
        Filter {
            should: Some(conditions),
            min_should: None,
            must: None,
            must_not: None,
        }
    }

    async fn create_and_load_disk_segment(
        test_dir: &TempDir,
        memory_segment: &Segment,
    ) -> anyhow::Result<Segment> {
        let paths = create_disk_segment(test_dir, memory_segment)?;
        unsafe_load_disk_segment(&paths).await
    }

    fn create_disk_segment(
        test_dir: &TempDir,
        memory_segment: &Segment,
    ) -> anyhow::Result<VectorDiskSegmentPaths> {
        let indexing_path = test_dir.path().join("indexing");
        fs::create_dir_all(&indexing_path)?;
        let disk_path = test_dir.path().join("disk");
        fs::create_dir_all(&disk_path)?;

        let disk_config = segment_config(DIMENSIONS, false, 4);
        Ok(build_disk_segment(memory_segment, &indexing_path, &disk_path, disk_config)?.paths)
    }

    fn search(segment: &Segment, vector: Vec<f32>) -> anyhow::Result<Vec<ExtendedPointId>> {
        search_with_index(
            segment,
            vector,
            &WithPayload {
                enable: false,
                payload_selector: None,
            },
        )
    }

    fn search_with_index(
        segment: &Segment,
        vector: Vec<f32>,
        payload: &WithPayload,
    ) -> anyhow::Result<Vec<ExtendedPointId>> {
        search_with_index_and_filter(segment, vector, payload, None)
    }

    fn search_with_index_and_filter(
        segment: &Segment,
        vector: Vec<f32>,
        payload: &WithPayload,
        filter: Option<&Filter>,
    ) -> anyhow::Result<Vec<ExtendedPointId>> {
        Ok(segment
            .search(
                DEFAULT_VECTOR_NAME,
                &QueryVector::Nearest(Vector::Dense(vector)),
                payload,
                &WithVector::Bool(false),
                filter,
                100,
                None,
                &AtomicBool::new(false),
            )?
            .into_iter()
            .map(|result| result.id)
            .collect())
    }

    #[tokio::test]
    async fn disk_segment_with_all_none_filter() -> anyhow::Result<()> {
        let num_vectors: usize = 10;
        let test_dir = tempfile::tempdir()?;
        let vectors: Vec<_> = stream_vectors_with_test_payload(num_vectors).collect();
        let (mut memory_segment, _) = create_test_memory_segment_with_payload(
            DIMENSIONS,
            &test_dir,
            vectors.clone().into_iter(),
        )?;
        create_test_payload_index(&mut memory_segment)?;

        let disk_segment = create_and_load_disk_segment(&test_dir, &memory_segment).await?;

        let with_payload = include_test_payload();
        let filter = Filter {
            should: None,
            min_should: None,
            must: None,
            must_not: None,
        };

        for (point_id, vector, _) in vectors {
            let results =
                search_with_index_and_filter(&disk_segment, vector, &with_payload, Some(&filter))?;
            let result_point_id = *results.first().context("Missing vector")?;
            assert_eq!(result_point_id, point_id)
        }
        Ok(())
    }

    #[tokio::test]
    async fn disk_segment_with_empty_filter() -> anyhow::Result<()> {
        let num_vectors: usize = 10;
        let test_dir = tempfile::tempdir()?;
        let vectors: Vec<_> = stream_vectors_with_test_payload(num_vectors).collect();
        let (mut memory_segment, _) = create_test_memory_segment_with_payload(
            DIMENSIONS,
            &test_dir,
            vectors.clone().into_iter(),
        )?;
        create_test_payload_index(&mut memory_segment)?;

        let disk_segment = create_and_load_disk_segment(&test_dir, &memory_segment).await?;

        let with_payload = WithPayload {
            enable: false,
            payload_selector: None,
        };

        let filter = Filter {
            should: Some(vec![]),
            min_should: None,
            must: None,
            must_not: None,
        };

        for (point_id, vector, _) in vectors {
            let results =
                search_with_index_and_filter(&disk_segment, vector, &with_payload, Some(&filter))?;
            let result_point_id = *results.first().context("Missing vector")?;
            assert_eq!(result_point_id, point_id)
        }
        Ok(())
    }

    #[test]
    fn plain_segment_with_empty_filter() -> anyhow::Result<()> {
        let num_vectors: usize = 10;
        let test_dir = tempfile::tempdir()?;
        let vectors: Vec<_> = stream_vectors_with_test_payload(num_vectors).collect();
        let (mut memory_segment, _) = create_test_memory_segment_with_payload(
            DIMENSIONS,
            &test_dir,
            vectors.clone().into_iter(),
        )?;
        create_test_payload_index(&mut memory_segment)?;

        let with_payload = include_test_payload();

        let filter = Filter {
            should: Some(vec![]),
            min_should: None,
            must: None,
            must_not: None,
        };
        for (point_id, vector, _) in vectors {
            let results = search_with_index_and_filter(
                &memory_segment,
                vector,
                &with_payload,
                Some(&filter),
            )?;
            let result_point_id = *results.first().context("Missing vector")?;
            assert_eq!(result_point_id, point_id)
        }
        Ok(())
    }

    #[tokio::test]
    async fn disk_segment_with_filter() -> anyhow::Result<()> {
        let num_vectors: usize = 10;
        let test_dir = tempfile::tempdir()?;
        let vectors: Vec<_> = stream_vectors_with_test_payload(num_vectors).collect();
        let (mut memory_segment, _) = create_test_memory_segment_with_payload(
            DIMENSIONS,
            &test_dir,
            vectors.clone().into_iter(),
        )?;
        create_test_payload_index(&mut memory_segment)?;

        let disk_segment = create_and_load_disk_segment(&test_dir, &memory_segment).await?;
        let with_payload = include_test_payload();

        for (point_id, vector, _) in vectors {
            let filter = test_payload_should_equal(point_id);

            let results =
                search_with_index_and_filter(&disk_segment, vector, &with_payload, Some(&filter))?;
            let result_point_id = *results.first().context("Missing vector")?;
            assert_eq!(result_point_id, point_id)
        }
        Ok(())
    }

    #[test]
    fn plain_segment_with_filter() -> anyhow::Result<()> {
        let num_vectors: usize = 10;
        let test_dir = tempfile::tempdir()?;
        let vectors: Vec<_> = stream_vectors_with_test_payload(num_vectors).collect();
        let (mut memory_segment, _) = create_test_memory_segment_with_payload(
            DIMENSIONS,
            &test_dir,
            vectors.clone().into_iter(),
        )?;
        create_test_payload_index(&mut memory_segment)?;

        let with_payload = include_test_payload();
        for (point_id, vector, _) in vectors {
            let filter = test_payload_should_equal(point_id);

            let results = search_with_index_and_filter(
                &memory_segment,
                vector,
                &with_payload,
                Some(&filter),
            )?;
            let result_point_id = *results.first().context("Missing vector")?;
            assert_eq!(result_point_id, point_id)
        }
        Ok(())
    }

    #[test]
    fn plain_segment_with_payload() -> anyhow::Result<()> {
        let num_vectors: usize = 10;
        let test_dir = tempfile::tempdir()?;
        let vectors: Vec<_> = stream_vectors_with_test_payload(num_vectors).collect();
        let (mut memory_segment, _) = create_test_memory_segment_with_payload(
            DIMENSIONS,
            &test_dir,
            vectors.clone().into_iter(),
        )?;
        create_test_payload_index(&mut memory_segment)?;

        let with_payload = include_test_payload();

        for (point_id, vector, _) in vectors {
            let results = search_with_index(&memory_segment, vector, &with_payload)?;
            let result_point_id = *results.first().context("Missing vector")?;
            assert_eq!(result_point_id, point_id)
        }
        Ok(())
    }

    #[tokio::test]
    async fn disk_segment_with_payload() -> anyhow::Result<()> {
        let num_vectors: usize = 10;
        let test_dir = tempfile::tempdir()?;
        let vectors: Vec<_> = stream_vectors_with_test_payload(num_vectors).collect();
        let (mut memory_segment, _) = create_test_memory_segment_with_payload(
            DIMENSIONS,
            &test_dir,
            vectors.clone().into_iter(),
        )?;
        create_test_payload_index(&mut memory_segment)?;

        let disk_segment = create_and_load_disk_segment(&test_dir, &memory_segment).await?;

        let with_payload = include_test_payload();

        for (point_id, vector, _) in vectors {
            let results = search_with_index(&disk_segment, vector, &with_payload)?;
            let result_point_id = *results.first().context("Missing vector")?;
            assert_eq!(result_point_id, point_id)
        }
        Ok(())
    }

    #[tokio::test]
    async fn disk_segment_can_be_opened_and_queried_with_different_id_trackers_and_bitsets(
    ) -> anyhow::Result<()> {
        let num_vectors: usize = 10;
        let test_dir = tempfile::tempdir()?;
        let vectors: Vec<_> = stream_vectors_with_test_payload(num_vectors).collect();
        let (mut memory_segment, id_tracker) = create_test_memory_segment_with_payload(
            DIMENSIONS,
            &test_dir,
            vectors.clone().into_iter(),
        )?;
        create_test_payload_index(&mut memory_segment)?;

        let paths_without_deletes = create_disk_segment(&test_dir, &memory_segment)?;
        let paths_with_deletes = paths_without_deletes.clone();

        // Write the bitset after the segment so that we do not delete the vectors from
        // storage when creating the segment.
        let mut deleted_bitset = DeletedBitset::load_from_path(paths_with_deletes.deleted_bitset)?;
        let mut deleted_ids = vec![];
        for (external_id, ..) in &vectors[..vectors.len() / 2] {
            let internal_id = id_tracker
                .borrow()
                .internal_id(*external_id)
                .context("Missing internal id?")?;
            deleted_bitset.delete(internal_id)?;
            deleted_ids.push(external_id);
        }
        let mutated_bitset_path = test_dir.path().join("mutated_bitset");
        {
            let mut file = BufWriter::new(File::create(mutated_bitset_path.clone())?);
            deleted_bitset.write(&mut file)?;
            file.into_inner()?.sync_all()?;
        }
        let paths_with_deletes = VectorDiskSegmentPaths {
            deleted_bitset: mutated_bitset_path,
            ..paths_with_deletes
        };

        let segment_without_deletes = unsafe_load_disk_segment(&paths_without_deletes).await?;
        let segment_with_deletes = unsafe_load_disk_segment(&paths_with_deletes).await?;

        let with_payload = include_test_payload();

        for (point_id, vector, _) in &vectors {
            let results =
                search_with_index(&segment_without_deletes, vector.clone(), &with_payload)?;
            let result_point_id = *results.first().context("Missing vector")?;
            assert_eq!(result_point_id, *point_id);

            let results = search_with_index(&segment_with_deletes, vector.clone(), &with_payload)?;

            if deleted_ids.contains(&point_id) {
                let result_point_id = *results.first().context("Missing vector")?;
                assert_ne!(result_point_id, *point_id);
            } else {
                let result_point_id = *results.first().context("Missing vector")?;
                assert_eq!(result_point_id, *point_id);
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn disk_segment_can_be_opened_multiple_times_and_queried_concurrently(
    ) -> anyhow::Result<()> {
        let num_vectors: usize = 100;
        let test_dir = tempfile::tempdir()?;
        let vectors: Vec<_> = stream_vectors_with_test_payload(num_vectors).collect();
        let (mut memory_segment, _) = create_test_memory_segment_with_payload(
            DIMENSIONS,
            &test_dir,
            vectors.clone().into_iter(),
        )?;
        create_test_payload_index(&mut memory_segment)?;

        let paths = create_disk_segment(&test_dir, &memory_segment)?;
        let disk_segment1 = unsafe_load_disk_segment(&paths).await?;
        let disk_segment2 = unsafe_load_disk_segment(&paths).await?;

        let with_payload = include_test_payload();

        let vectors_clone = vectors.clone();
        let payload_clone = with_payload.clone();
        let query_1 =
            async move { search_for_all_vectors(disk_segment1, vectors_clone, payload_clone) };
        let query_2 = async move {
            search_for_all_vectors(disk_segment2, vectors.clone(), with_payload.clone())
        };
        let (first, second) = try_join!(tokio::spawn(query_1), tokio::spawn(query_2))?;
        first?;
        second?;
        Ok(())
    }

    fn search_for_all_vectors(
        segment: Segment,
        vectors: Vec<(ExtendedPointId, Vec<f32>, JsonValue)>,
        with_payload: WithPayload,
    ) -> anyhow::Result<()> {
        for (point_id, vector, _) in vectors {
            let results = search_with_index(&segment, vector, &with_payload)?;
            let result_point_id = *results.first().context("Missing vector")?;
            assert_eq!(result_point_id, point_id);
        }
        Ok(())
    }

    #[test]
    fn plain_segment_can_delete_points() -> anyhow::Result<()> {
        let num_vectors: usize = 10;
        let test_dir = tempfile::tempdir()?;
        let vectors: Vec<_> = stream_vectors(num_vectors).collect();
        let (mut memory_segment, _) =
            create_test_memory_segment(DIMENSIONS, &test_dir, vectors.clone().into_iter())?;
        let to_delete: Vec<_> = vectors.iter().take(num_vectors / 2).cloned().collect();

        for (point_id, _) in &to_delete {
            assert!(memory_segment.delete_point(OP_NUM, *point_id)?);
        }

        for id_and_vector in vectors {
            let expect_delete = to_delete.contains(&id_and_vector);
            let (point_id, vector) = id_and_vector;
            let results = search(&memory_segment, vector)?;
            let result_point_id = *results.first().context("Missing vector")?;
            if expect_delete {
                assert_ne!(result_point_id, point_id);
            } else {
                assert_eq!(result_point_id, point_id);
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn can_load_plain_segment_from_snapshot() -> anyhow::Result<()> {
        let num_vectors: usize = 10;
        let test_dir = tempfile::tempdir()?;
        let vectors: Vec<_> = stream_vectors(num_vectors).collect();
        let (memory_segment, id_tracker) =
            create_test_memory_segment(DIMENSIONS, &test_dir, vectors.clone().into_iter())?;

        let indexing_path = test_dir.path().join("indexing");
        fs::create_dir_all(&indexing_path)?;
        let disk_path = test_dir.path().join("disk");
        fs::create_dir_all(&disk_path)?;

        let paths = snapshot_segment(&id_tracker, &memory_segment, &indexing_path, &disk_path)?;

        let segment = unsafe_load_disk_segment(&paths).await?;

        for vector in vectors {
            let results = search(&segment, vector.1)?;
            assert_eq!(*results.first().expect("Missing vector!"), vector.0);
        }
        Ok(())
    }

    #[tokio::test]
    async fn plain_segment_with_deleted_points_can_be_written_then_queried() -> anyhow::Result<()> {
        let num_vectors: usize = 10;
        let test_dir = tempfile::tempdir()?;
        let vectors: Vec<_> = stream_vectors(num_vectors).collect();
        let (mut memory_segment, id_tracker) =
            create_test_memory_segment(DIMENSIONS, &test_dir, vectors.clone().into_iter())?;
        let to_delete: Vec<_> = vectors.iter().take(num_vectors / 2).cloned().collect();

        for (point_id, _) in &to_delete {
            assert!(memory_segment.delete_point(OP_NUM, *point_id)?);
        }

        let indexing_path = test_dir.path().join("indexing");
        fs::create_dir_all(&indexing_path)?;
        let disk_path = test_dir.path().join("disk");
        fs::create_dir_all(&disk_path)?;
        let paths = snapshot_segment(&id_tracker, &memory_segment, &indexing_path, &disk_path)?;

        let segment = unsafe_load_disk_segment(&paths).await?;

        for id_and_vector in vectors {
            let expect_delete = to_delete.contains(&id_and_vector);
            let (point_id, vector) = id_and_vector;
            let results = search(&segment, vector)?;

            if expect_delete {
                assert!(!results.contains(&point_id));
            } else {
                assert!(results.contains(&point_id));
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn plain_segment_with_deleted_points_can_be_written_to_hnsw_and_queried(
    ) -> anyhow::Result<()> {
        let num_vectors: usize = 10;
        let test_dir = tempfile::tempdir()?;
        let vectors: Vec<_> = stream_vectors(num_vectors).collect();
        let (mut memory_segment, _) =
            create_test_memory_segment(DIMENSIONS, &test_dir, vectors.clone().into_iter())?;
        let to_delete: Vec<_> = vectors.iter().take(num_vectors / 2).cloned().collect();

        for (point_id, _) in &to_delete {
            assert!(memory_segment.delete_point(OP_NUM, *point_id)?);
        }

        let disk_segment = create_and_load_disk_segment(&test_dir, &memory_segment).await?;

        for id_and_vector in vectors {
            let expect_delete = to_delete.contains(&id_and_vector);
            let (point_id, vector) = id_and_vector;
            let results = search(&disk_segment, vector)?;

            if expect_delete {
                assert!(!results.contains(&point_id));
            } else {
                assert!(results.contains(&point_id));
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn plain_segment_with_deleted_points_via_id_tracker_can_be_written_to_hnsw_and_queried(
    ) -> anyhow::Result<()> {
        let num_vectors: usize = 10;
        let test_dir = tempfile::tempdir()?;
        let vectors: Vec<_> = stream_vectors(num_vectors).collect();
        let (memory_segment, id_tracker) =
            create_test_memory_segment(DIMENSIONS, &test_dir, vectors.clone().into_iter())?;
        let to_delete: Vec<_> = vectors.iter().take(num_vectors / 2).cloned().collect();

        for (point_id, _) in &to_delete {
            id_tracker.borrow_mut().drop(*point_id)?;
        }

        let disk_segment = create_and_load_disk_segment(&test_dir, &memory_segment).await?;

        for id_and_vector in vectors {
            let expect_delete = to_delete.contains(&id_and_vector);
            let (point_id, vector) = id_and_vector;
            let results = search(&disk_segment, vector)?;

            if expect_delete {
                assert!(!results.contains(&point_id));
            } else {
                assert!(results.contains(&point_id));
            }
        }
        Ok(())
    }

    #[tokio::test]
    async fn qdrant_2_with_deleted_bitset_ignores_deleted_ids() -> anyhow::Result<()> {
        let num_vectors: usize = 10;
        let test_dir = tempfile::tempdir()?;

        // Create a disk segment
        let vectors: Vec<_> = stream_vectors(num_vectors).collect();
        let to_delete = vectors.first().expect("Created 0 vectors?").clone();
        let disk_segment_paths =
            create_test_disk_segment(DIMENSIONS, &test_dir, vectors.clone().into_iter())?;

        // Mutate the deleted bitset and write it back to disk, as we would expect from
        // the index worker.
        let VectorDiskSegmentPaths {
            deleted_bitset: deleted_bitset_path,
            uuids,
            ..
        } = &disk_segment_paths;

        // As if we were in the index worker, load the id tracker and bitset files.
        let id_tracker = VectorStaticIdTracker {
            id_tracker: StaticIdTracker::load_from_path(uuids.clone())?,
            deleted_bitset: DeletedBitset::new(vectors.len()),
        };

        // Then delete a vector.
        let mut deleted_bitset = DeletedBitset::load_from_path(deleted_bitset_path.clone())?;
        let internal_id_to_delete = id_tracker
            .internal_id(to_delete.0)
            .expect("Missing internal id");
        deleted_bitset.delete(internal_id_to_delete)?;
        deleted_bitset.write_to_path(deleted_bitset_path.clone())?;

        // Now as if we're on searchlight, load the id tracker with the updated bitset.
        // And Load the segment from disk, ensuring that it ignores the vector we just
        // marked as deleted.
        let disk_segment = unsafe_load_disk_segment(&disk_segment_paths).await?;
        let results = search(&disk_segment, to_delete.1)?;
        assert!(!results.contains(&to_delete.0));

        Ok(())
    }

    fn merge_disk_segments_tmpdir(
        segments: Vec<&Segment>,
        tmp_dir: &TempDir,
        config: SegmentConfig,
    ) -> anyhow::Result<VectorDiskSegmentValues> {
        let indexing_path = tmp_dir.path().join("indexing");
        fs::create_dir_all(&indexing_path)?;
        let disk_path = tmp_dir.path().join("disk");
        fs::create_dir_all(&disk_path)?;
        merge_disk_segments(
            segments.into_iter().map(|s| (None, s)).collect(),
            &indexing_path,
            &disk_path,
            config,
        )
    }

    // One way this might happen is if we accidentally have the same vector in a
    // non-deleted state in multiple segments in the same index. Easy to do in
    // tests, but it should never happen in prod. If a document is inserted or
    // updated all old versions should have been marked deleted.
    #[tokio::test]
    async fn merge_segments_with_same_vector_fails() -> anyhow::Result<()> {
        let vector: Vec<_> = stream_vectors(1).collect();

        let initial_dir = tempfile::tempdir()?;
        let initial_paths =
            create_test_disk_segment(DIMENSIONS, &initial_dir, vector.clone().into_iter())?;
        let initial_segment = unsafe_load_disk_segment(&initial_paths).await?;

        let new_dir = tempfile::tempdir()?;
        let new_paths = create_test_disk_segment(DIMENSIONS, &new_dir, vector.into_iter())?;
        let new_segment = unsafe_load_disk_segment(&new_paths).await?;

        let config = segment_config(DIMENSIONS, false, 4);
        let merged_dir = tempfile::tempdir()?;
        let result =
            merge_disk_segments_tmpdir(vec![&initial_segment, &new_segment], &merged_dir, config)
                .expect_err("Created invalid merged segment!");
        assert_eq!(
            result.to_string(),
            "Condition failed: `vector_count == total_point_count` (2 vs 1)"
        );
        Ok(())
    }

    #[tokio::test]
    async fn merge_segments_includes_payload_index() -> anyhow::Result<()> {
        let vectors: Vec<_> = stream_vectors(10).collect();

        let initial_dir = tempfile::tempdir()?;
        let initial_paths =
            create_test_disk_segment(DIMENSIONS, &initial_dir, vectors.into_iter())?;
        let initial_segment = unsafe_load_disk_segment(&initial_paths).await?;

        let vectors: Vec<_> = stream_vectors(10).collect();
        let new_dir = tempfile::tempdir()?;
        let new_paths = create_test_disk_segment(DIMENSIONS, &new_dir, vectors.into_iter())?;
        let new_segment = unsafe_load_disk_segment(&new_paths).await?;

        let config = segment_config(DIMENSIONS, false, 4);
        let merged_dir = tempfile::tempdir()?;
        let VectorDiskSegmentValues { paths, .. } =
            merge_disk_segments_tmpdir(vec![&initial_segment, &new_segment], &merged_dir, config)?;
        let merged_segment = unsafe_load_disk_segment(&paths).await?;
        assert_eq!(
            merged_segment.get_indexed_fields(),
            initial_segment.get_indexed_fields()
        );

        Ok(())
    }

    #[tokio::test]
    async fn merge_segments_with_same_vector_where_one_copy_is_deleted_includes_the_vector(
    ) -> anyhow::Result<()> {
        let vector: Vec<_> = stream_vectors(1).collect();

        let initial_dir = tempfile::tempdir()?;
        let initial_paths =
            create_test_disk_segment(DIMENSIONS, &initial_dir, vector.clone().into_iter())?;

        let mut deleted_bitset =
            DeletedBitset::load_from_path(initial_paths.deleted_bitset.clone())?;
        let id_tracker = VectorStaticIdTracker {
            id_tracker: StaticIdTracker::load_from_path(initial_paths.uuids.clone())?,
            deleted_bitset: deleted_bitset.clone(),
        };
        let internal_id_to_delete = id_tracker
            .internal_id(vector.first().unwrap().0)
            .expect("Missing internal id");
        deleted_bitset.delete(internal_id_to_delete)?;
        deleted_bitset.write_to_path(initial_paths.deleted_bitset.clone())?;
        let initial_segment = unsafe_load_disk_segment(&initial_paths).await?;

        let new_dir = tempfile::tempdir()?;
        let new_paths = create_test_disk_segment(DIMENSIONS, &new_dir, vector.clone().into_iter())?;
        let new_segment = unsafe_load_disk_segment(&new_paths).await?;

        let config = segment_config(DIMENSIONS, false, 4);
        let merged_dir = tempfile::tempdir()?;
        let VectorDiskSegmentValues {
            paths: merged_paths,
            num_vectors,
            ..
        } = merge_disk_segments_tmpdir(vec![&initial_segment, &new_segment], &merged_dir, config)?;
        let merged = unsafe_load_disk_segment(&merged_paths).await?;

        let (point_id, vector) = vector.into_iter().next().unwrap();
        let with_payload = include_test_payload();
        let results = search_with_index(&merged, vector, &with_payload)?;
        let result_point_id = *results.first().context("Missing vector")?;
        assert_eq!(result_point_id, point_id);
        assert_eq!(1, num_vectors);

        Ok(())
    }

    #[tokio::test]
    async fn merge_segments_without_deletes_contains_all_vectors() -> anyhow::Result<()> {
        let num_vectors: usize = 10;
        let num_segments = 3;

        let mut segments_dirs_vecs = vec![];
        for _ in 0..num_segments {
            let tmp_dir = tempfile::tempdir()?;
            let vectors: Vec<_> = stream_vectors(num_vectors).collect();
            let paths =
                create_test_disk_segment(DIMENSIONS, &tmp_dir, vectors.clone().into_iter())?;
            let segment = unsafe_load_disk_segment(&paths).await?;
            segments_dirs_vecs.push((segment, tmp_dir, vectors));
        }

        let segments: Vec<&Segment> = segments_dirs_vecs
            .iter()
            .map(|(segment, ..)| segment)
            .collect();

        let config = segment_config(DIMENSIONS, false, 4);
        let merged_dir = tempfile::tempdir()?;
        let VectorDiskSegmentValues {
            paths: merged_paths,
            num_vectors: merged_num_vectors,
            ..
        } = merge_disk_segments_tmpdir(segments, &merged_dir, config)?;
        let merged = unsafe_load_disk_segment(&merged_paths).await?;

        let vectors: Vec<_> = segments_dirs_vecs
            .iter()
            .flat_map(|(_, _, vectors)| vectors.clone())
            .collect();

        let with_payload = include_test_payload();
        for (point_id, vector) in vectors {
            let results = search_with_index(&merged, vector, &with_payload)?;
            let result_point_id = *results.first().context("Missing vector")?;
            assert_eq!(result_point_id, point_id);
        }
        assert_eq!((num_vectors * num_segments) as u32, merged_num_vectors);
        Ok(())
    }

    #[tokio::test]
    async fn merge_segments_with_deletes_drops_deleted_vectors() -> anyhow::Result<()> {
        let num_vectors: usize = 10;
        let test_dir = tempfile::tempdir()?;
        let vectors: Vec<_> = stream_vectors(num_vectors).collect();
        let (mut segment_with_deletes, _) =
            create_test_memory_segment(DIMENSIONS, &test_dir, vectors.clone().into_iter())?;
        let to_delete: Vec<_> = vectors.iter().take(num_vectors / 2).cloned().collect();

        for (point_id, _) in &to_delete {
            assert!(segment_with_deletes.delete_point(OP_NUM, *point_id)?);
        }

        let other_dir = tempfile::tempdir()?;
        let other_vectors: Vec<_> = stream_vectors(num_vectors).collect();
        let other_paths =
            create_test_disk_segment(DIMENSIONS, &other_dir, other_vectors.clone().into_iter())?;
        let other_segment = unsafe_load_disk_segment(&other_paths).await?;

        let config = segment_config(DIMENSIONS, false, 4);
        let merged_dir = tempfile::tempdir()?;
        let VectorDiskSegmentValues {
            paths: merged_paths,
            num_vectors: merged_num_vectors,
            ..
        } = merge_disk_segments_tmpdir(
            vec![&segment_with_deletes, &other_segment],
            &merged_dir,
            config,
        )?;
        let merged = unsafe_load_disk_segment(&merged_paths).await?;

        let with_payload = include_test_payload();
        for id_and_vec in vectors.into_iter().chain(other_vectors) {
            let is_deleted = to_delete.contains(&id_and_vec);
            let (point_id, vector) = id_and_vec;
            let results = search_with_index(&merged, vector, &with_payload)?;
            let result_point_id = *results.first().context("Missing vector")?;
            if is_deleted {
                assert_ne!(result_point_id, point_id);
            } else {
                assert_eq!(result_point_id, point_id);
            }
        }
        // 1 Full segment + 1 50% deleted segment.
        assert_eq!((num_vectors + num_vectors / 2) as u32, merged_num_vectors);
        Ok(())
    }
}
