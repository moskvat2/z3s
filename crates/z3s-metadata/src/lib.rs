//! # z3s-metadata
//!
//! Catálogo de metadados distribuído, consenso Raft, particionamento lexical e Witness Cache.

pub mod command;
pub mod partition;
pub mod service;
pub mod state_machine;
pub mod witness_cache;

pub use command::{MetadataCommand, MetadataLogEntry};
pub use partition::{KeyRange, MetadataPartition, PartitionId, PartitionRouter};
pub use service::DistributedMetadataService;
pub use state_machine::{MetadataSnapshot, MetadataStateMachine};
pub use witness_cache::{WitnessCache, WitnessEntry};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;
    use z3s_common::manifest::{ObjectManifest, ObjectMetadata};
    use z3s_common::types::{BucketName, ETag, ObjectKey, VersionId};

    fn make_test_manifest(bucket: &str, key: &str, size: u64) -> ObjectManifest {
        ObjectManifest {
            metadata: ObjectMetadata {
                bucket: BucketName::new(bucket).unwrap(),
                key: ObjectKey::new(key).unwrap(),
                version_id: VersionId::new("null"),
                size,
                etag: ETag::from_hex("d41d8cd98f00b204e9800998ecf8427e"),
                content_type: "application/octet-stream".to_string(),
                storage_class: z3s_common::manifest::StorageClass::Standard,
                created_at: Utc::now(),
                user_metadata: HashMap::new(),
                merkle_root: [0u8; 32],
                is_delete_marker: false,
                is_latest: true,
                encryption: None,
            },
            data_shards_count: 4,
            parity_shards_count: 2,
            parts: Vec::new(),
            shards: Vec::new(),
        }
    }

    #[test]
    fn test_metadata_state_machine_lifecycle() {
        let mut sm = MetadataStateMachine::new();

        // 1. Cria Bucket
        sm.apply(
            1,
            1,
            MetadataCommand::CreateBucket {
                bucket: "teste-bucket".to_string(),
                created_at: Utc::now(),
            },
        );
        assert!(sm.bucket_exists("teste-bucket"));

        // 2. Insere Objeto
        let m1 = make_test_manifest("teste-bucket", "docs/relatorio.pdf", 1024);
        sm.apply(2, 1, MetadataCommand::PutObjectManifest { manifest: m1 });
        assert!(sm.get_object("teste-bucket", "docs/relatorio.pdf").is_some());

        // 3. Renomeia Objeto
        sm.apply(
            3,
            1,
            MetadataCommand::RenameObjectManifest {
                src_bucket: "teste-bucket".to_string(),
                src_key: "docs/relatorio.pdf".to_string(),
                dest_bucket: "teste-bucket".to_string(),
                dest_key: "docs/final.pdf".to_string(),
            },
        );
        assert!(sm.get_object("teste-bucket", "docs/relatorio.pdf").is_none());
        assert!(sm.get_object("teste-bucket", "docs/final.pdf").is_some());

        // 4. Snapshot & Restore
        let snapshot = sm.create_snapshot();
        let mut sm2 = MetadataStateMachine::new();
        sm2.restore_snapshot(snapshot);
        assert_eq!(sm2.last_applied_index(), 3);
        assert!(sm2.get_object("teste-bucket", "docs/final.pdf").is_some());
    }

    #[test]
    fn test_partition_router_lexicographical_routing() {
        let mut router = PartitionRouter::new();

        // Partição para [a, m)
        router.add_partition(MetadataPartition {
            id: PartitionId::new_v4(),
            range: KeyRange {
                start_key: "bucket/a".to_string(),
                end_key: Some("bucket/m".to_string()),
            },
            leader_node_id: Some(Uuid::new_v4()),
            replica_node_ids: vec![Uuid::new_v4()],
        });

        // Partição para [m, z)
        router.add_partition(MetadataPartition {
            id: PartitionId::new_v4(),
            range: KeyRange {
                start_key: "bucket/m".to_string(),
                end_key: Some("bucket/z".to_string()),
            },
            leader_node_id: Some(Uuid::new_v4()),
            replica_node_ids: vec![Uuid::new_v4()],
        });

        let p_docs = router.route("bucket/docs/plan.txt").unwrap();
        assert_eq!(p_docs.range.start_key, "bucket/a");

        let p_reports = router.route("bucket/reports/q4.pdf").unwrap();
        assert_eq!(p_reports.range.start_key, "bucket/m");
    }

    #[test]
    fn test_distributed_metadata_service_with_witness_cache() {
        let service = DistributedMetadataService::new();

        service
            .commit_command(MetadataCommand::CreateBucket {
                bucket: "prod-bucket".to_string(),
                created_at: Utc::now(),
            })
            .unwrap();

        let manifest = make_test_manifest("prod-bucket", "foto.jpg", 2048);
        service
            .commit_command(MetadataCommand::PutObjectManifest { manifest })
            .unwrap();

        // Primeira leitura popula o witness cache
        let obj1 = service.get_object("prod-bucket", "foto.jpg");
        assert!(obj1.is_some());
        assert_eq!(obj1.unwrap().metadata.size, 2048);

        // Deleta objeto
        service
            .commit_command(MetadataCommand::DeleteObjectManifest {
                bucket: "prod-bucket".to_string(),
                key: "foto.jpg".to_string(),
            })
            .unwrap();

        // Leitura pós-delete linearizável no witness cache
        let obj2 = service.get_object("prod-bucket", "foto.jpg");
        assert!(obj2.is_none());
    }
}
