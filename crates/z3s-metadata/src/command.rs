use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use z3s_common::manifest::ObjectManifest;

/// Comandos atômicos replicados no Log de Consenso de Metadados
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetadataCommand {
    /// Criação de um novo Bucket
    CreateBucket {
        bucket: String,
        created_at: DateTime<Utc>,
    },
    /// Exclusão de um Bucket
    DeleteBucket {
        bucket: String,
    },
    /// Gravação ou atualização atômica de manifesto de objeto
    PutObjectManifest {
        manifest: ObjectManifest,
    },
    /// Exclusão de manifesto de objeto
    DeleteObjectManifest {
        bucket: String,
        key: String,
    },
    /// Renomeação / movimentação atômica de objeto
    RenameObjectManifest {
        src_bucket: String,
        src_key: String,
        dest_bucket: String,
        dest_key: String,
    },
}

/// Entrada estruturada no Log de Consenso
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataLogEntry {
    pub index: u64,
    pub term: u64,
    pub command: MetadataCommand,
    pub timestamp: DateTime<Utc>,
    pub checksum_crc32: u32,
}

impl MetadataLogEntry {
    pub fn new(index: u64, term: u64, command: MetadataCommand) -> Self {
        let timestamp = Utc::now();
        let mut entry = Self {
            index,
            term,
            command,
            timestamp,
            checksum_crc32: 0,
        };
        entry.checksum_crc32 = entry.compute_checksum();
        entry
    }

    pub fn compute_checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.index.to_le_bytes());
        hasher.update(&self.term.to_le_bytes());
        hasher.update(&self.timestamp.timestamp_nanos_opt().unwrap_or(0).to_le_bytes());
        if let Ok(bytes) = serde_json::to_vec(&self.command) {
            hasher.update(&bytes);
        }
        hasher.finalize()
    }

    pub fn is_valid(&self) -> bool {
        self.checksum_crc32 == self.compute_checksum()
    }
}
