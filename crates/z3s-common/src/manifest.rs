use crate::types::{BucketName, ETag, ObjectKey, ShardId, VersionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Classe de Armazenamento compatível com S3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageClass {
    Standard,
    ReducedRedundancy,
    StandardIa,
    Glacier,
}

impl Default for StorageClass {
    fn default() -> Self {
        Self::Standard
    }
}

fn default_true() -> bool {
    true
}

/// Metadados de criptografia em repouso (SSE-S3 / SSE-KMS / SSE-C)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectEncryptionMetadata {
    pub algorithm: String, // "AES256" | "aws:kms" | "SSE-C"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kms_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_dek_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dek_iv_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_iv_hex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_md5: Option<String>,
}

/// Metadados imutáveis do Objeto
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectMetadata {
    pub bucket: BucketName,
    pub key: ObjectKey,
    pub version_id: VersionId,
    pub size: u64,
    pub etag: ETag,
    pub content_type: String,
    pub storage_class: StorageClass,
    pub created_at: DateTime<Utc>,
    pub user_metadata: HashMap<String, String>,
    pub merkle_root: [u8; 32],
    #[serde(default)]
    pub is_delete_marker: bool,
    #[serde(default = "default_true")]
    pub is_latest: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encryption: Option<ObjectEncryptionMetadata>,
}

impl ObjectMetadata {
    pub fn new_delete_marker(bucket: BucketName, key: ObjectKey, version_id: VersionId) -> Self {
        Self {
            bucket,
            key,
            version_id,
            size: 0,
            etag: ETag::from_hex("d41d8cd98f00b204e9800998ecf8427e"),
            content_type: "application/octet-stream".to_string(),
            storage_class: StorageClass::Standard,
            created_at: Utc::now(),
            user_metadata: HashMap::new(),
            merkle_root: [0u8; 32],
            is_delete_marker: true,
            is_latest: true,
            encryption: None,
        }
    }
}

/// Ponteiro físico para localização de um Shard gravado em um Extent File
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardPointer {
    pub shard_id: ShardId,
    pub shard_index: u32,
    pub is_parity: bool,
    pub node_id: Uuid,
    pub extent_id: Uuid,
    pub offset_in_extent: u64,
    pub length: u64,
    pub blake3_checksum: [u8; 32],
}

/// Manifesto de uma parte de upload (usado tanto em uploads simples quanto multipart)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartManifest {
    pub part_number: u32,
    pub size: u64,
    pub etag: ETag,
    pub shards: Vec<ShardPointer>,
}

/// Manifesto completo que mapeia o objeto lógico aos seus shards físicos distribuídos
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectManifest {
    pub metadata: ObjectMetadata,
    pub data_shards_count: u32,
    pub parity_shards_count: u32,
    #[serde(default)]
    pub parts: Vec<PartManifest>,
    pub shards: Vec<ShardPointer>,
}

impl ObjectManifest {
    pub fn new(
        metadata: ObjectMetadata,
        data_shards_count: u32,
        parity_shards_count: u32,
        shards: Vec<ShardPointer>,
    ) -> Self {
        Self {
            metadata,
            data_shards_count,
            parity_shards_count,
            parts: Vec::new(),
            shards,
        }
    }

    pub fn new_with_parts(
        metadata: ObjectMetadata,
        data_shards_count: u32,
        parity_shards_count: u32,
        parts: Vec<PartManifest>,
    ) -> Self {
        let mut all_shards = Vec::new();
        for p in &parts {
            all_shards.extend(p.shards.clone());
        }
        Self {
            metadata,
            data_shards_count,
            parity_shards_count,
            parts,
            shards: all_shards,
        }
    }

    /// Retorna os shards de dados (não-paridade)
    pub fn data_shards(&self) -> impl Iterator<Item = &ShardPointer> {
        self.shards.iter().filter(|s| !s.is_parity)
    }

    /// Retorna os shards de paridade
    pub fn parity_shards(&self) -> impl Iterator<Item = &ShardPointer> {
        self.shards.iter().filter(|s| s.is_parity)
    }
}
