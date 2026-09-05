use serde::{Deserialize, Serialize};
use std::fmt;

/// Nome de Bucket do S3 com validação estrita de DNS (3-63 chars, minúsculas, números, hífens)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BucketName(String);

impl BucketName {
    pub fn new(name: impl Into<String>) -> Result<Self, &'static str> {
        let name = name.into();
        if name.len() < 3 || name.len() > 63 {
            return Err("O nome do bucket deve ter entre 3 e 63 caracteres");
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.')
        {
            return Err("O nome do bucket contém caracteres inválidos");
        }
        if name.starts_with('.') || name.starts_with('-') || name.ends_with('.') || name.ends_with('-') {
            return Err("O nome do bucket não pode começar ou terminar com ponto ou hífen");
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BucketName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Chave do objeto S3 (Object Key)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectKey(String);

impl ObjectKey {
    pub fn new(key: impl Into<String>) -> Result<Self, &'static str> {
        let key = key.into();
        if key.is_empty() || key.len() > 1024 {
            return Err("A chave do objeto deve ter entre 1 e 1024 caracteres");
        }
        Ok(Self(key))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ObjectKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Identificador de Versão de Objeto
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VersionId(String);

impl VersionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Identificador único de Shard em disco
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShardId(pub uuid::Uuid);

impl ShardId {
    pub fn new_v7() -> Self {
        Self(uuid::Uuid::now_v7())
    }
}

/// Identificador único de Nó de Armazenamento no Cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub uuid::Uuid);

impl NodeId {
    pub fn new_v4() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

/// ETag oficial S3 (MD5 hex ou hash composto para multipart)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ETag(String);

impl ETag {
    pub fn from_hex(hex: impl Into<String>) -> Self {
        let hex = hex.into();
        let formatted = if hex.starts_with('"') && hex.ends_with('"') {
            hex
        } else {
            format!("\"{}\"", hex)
        };
        Self(formatted)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Intervalo de bytes solicitado via header HTTP Range
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    pub start: u64,
    pub end: Option<u64>,
}
