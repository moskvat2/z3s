//! # z3s-kms
//!
//! Criptografia em repouso (SSE-S3, SSE-C, SSE-KMS) com Envelope Encryption,
//! motor de avaliação de políticas IAM / Bucket Policies e WORM Object Lock.

pub mod envelope;
pub mod lock;
pub mod policy;

pub use envelope::{EncryptedDataKey, KmsEngine, KmsError};
pub use lock::{DefaultRetention, LegalHoldStatus, ObjectLockConfiguration, ObjectRetention, RetentionMode};
pub use policy::{BucketPolicy, PolicyAction, PolicyEffect, PolicyPrincipal, PolicyResource, Statement};
