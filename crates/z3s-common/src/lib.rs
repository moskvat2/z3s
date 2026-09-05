//! # z3s-common
//!
//! Tipos canônicos, erros padronizados compatíveis com a API S3 e primitivas criptográficas.

pub mod error;
pub mod hash;
pub mod manifest;
pub mod merkle;
pub mod types;

pub use error::{S3Error, S3ErrorCode, S3Result};
pub use hash::{blake3_hash, calculate_crc32c, calculate_md5_etag};
pub use manifest::{ObjectManifest, ObjectMetadata, ShardPointer, StorageClass};
pub use merkle::{MerkleProof, MerkleTree};
pub use types::{BucketName, ByteRange, ETag, ObjectKey, ShardId, VersionId};
