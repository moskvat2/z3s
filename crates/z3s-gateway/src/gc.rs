use std::collections::HashSet;
use std::sync::Arc;
use z3s_metadata::MetadataStateMachine;
use z3s_storage::{StorageEngine, StorageError};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GcReport {
    pub unreferenced_shards_deleted: usize,
    pub extents_compacted: usize,
    pub bytes_reclaimed: u64,
    pub stale_multipart_uploads_aborted: usize,
}

/// Coletor de Lixo Distribuído (Garbage Collector & Compaction Engine)
pub struct GarbageCollector {
    storage: Arc<StorageEngine>,
    metadata: Arc<MetadataStateMachine>,
}

impl GarbageCollector {
    pub fn new(storage: Arc<StorageEngine>, metadata: Arc<MetadataStateMachine>) -> Self {
        Self { storage, metadata }
    }

    /// Executa uma rodada completa de Coleta de Lixo:
    /// 1. Coleta e remove shards órfãos do disco não referenciados no catálogo de metadados
    /// 2. Executa a compactação de Extents selados para recuperar bytes físicos
    pub fn run_garbage_collection(&self) -> Result<GcReport, StorageError> {
        let mut report = GcReport::default();

        // 1. Constrói o conjunto de todos os Shard IDs ativos referenciados nos manifestos
        let mut active_shard_ids = HashSet::new();
        let all_manifests = self.metadata.list_all_manifests();
        for manifest in all_manifests {
            for shard in &manifest.shards {
                active_shard_ids.insert(shard.shard_id.0);
            }
            for part in &manifest.parts {
                for shard in &part.shards {
                    active_shard_ids.insert(shard.shard_id.0);
                }
            }
        }

        // 2. Compara com os shards gravados no disco local
        let disk_shards = self.storage.list_shards();
        for loc in disk_shards {
            if !active_shard_ids.contains(&loc.shard_id) {
                // Shard órfão detectado -> Deleta do disco
                self.storage.delete_shard(&loc.shard_id)?;
                report.unreferenced_shards_deleted += 1;
            }
        }

        // 3. Compacta os Extents selados após a limpeza de shards
        let compaction = self.storage.compact_sealed_extents()?;
        report.extents_compacted = compaction.extents_reclaimed;
        report.bytes_reclaimed = compaction.bytes_reclaimed;

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;
    use z3s_common::manifest::{ObjectManifest, ObjectMetadata, ShardPointer, StorageClass};
    use z3s_common::types::{BucketName, ETag, ObjectKey, ShardId, VersionId};

    #[test]
    fn test_garbage_collector_cleans_unreferenced_shards_and_compacts() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(StorageEngine::open(temp_dir.path(), 8192).unwrap());

        // 1. Grava 3 shards no storage
        let shard_1 = Uuid::new_v4();
        let shard_2 = Uuid::new_v4();
        let shard_3 = Uuid::new_v4();

        let loc_1 = storage.write_shard(shard_1, 0, 1, b"Live shard 1").unwrap();
        let _loc_2 = storage.write_shard(shard_2, 0, 1, b"Orphan shard 2").unwrap();
        let _loc_3 = storage.write_shard(shard_3, 0, 1, b"Orphan shard 3").unwrap();

        // 2. Registra no MetadataStateMachine apenas o manifesto com Shard 1
        let obj_meta = ObjectMetadata {
            bucket: BucketName::new("gc-bucket").unwrap(),
            key: ObjectKey::new("file.txt").unwrap(),
            version_id: VersionId::new("v1"),
            size: 12,
            etag: ETag::from_hex("0123456789abcdef0123456789abcdef"),
            content_type: "text/plain".to_string(),
            storage_class: StorageClass::Standard,
            created_at: Utc::now(),
            user_metadata: Default::default(),
            merkle_root: [0u8; 32],
            is_delete_marker: false,
            is_latest: true,
            encryption: None,
        };

        let manifest = ObjectManifest::new(
            obj_meta,
            1,
            0,
            vec![ShardPointer {
                shard_id: ShardId(shard_1),
                shard_index: 0,
                is_parity: false,
                node_id: Uuid::new_v4(),
                extent_id: loc_1.extent_id,
                offset_in_extent: loc_1.offset_in_extent,
                length: 12,
                blake3_checksum: [0u8; 32],
            }],
        );

        let mut sm = MetadataStateMachine::new();
        sm.apply(1, 1, z3s_metadata::MetadataCommand::PutObjectManifest { manifest });
        let gc = GarbageCollector::new(storage.clone(), Arc::new(sm));

        // 3. Executa o Garbage Collector
        let report = gc.run_garbage_collection().unwrap();

        // Shards 2 e 3 devem ter sido deletados como órfãos
        assert_eq!(report.unreferenced_shards_deleted, 2);

        // 4. Valida que Shard 1 continua íntegro e Shards 2 e 3 foram expurgados
        assert!(storage.read_shard(&shard_1).is_ok());
        assert!(storage.read_shard(&shard_2).is_err());
        assert!(storage.read_shard(&shard_3).is_err());
    }
}
