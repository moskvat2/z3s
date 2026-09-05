use crate::command::MetadataCommand;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use z3s_common::manifest::ObjectManifest;

/// Snapshot determinístico do estado do catálogo de metadados
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataSnapshot {
    pub last_applied_index: u64,
    pub last_applied_term: u64,
    pub buckets: HashMap<String, DateTime<Utc>>,
    pub objects: BTreeMap<String, ObjectManifest>,
}

/// Máquina de Estados Determinística de Metadados (Replicated State Machine)
#[derive(Debug, Clone, Default)]
pub struct MetadataStateMachine {
    last_applied_index: u64,
    last_applied_term: u64,
    buckets: HashMap<String, DateTime<Utc>>,
    // Chave no formato "bucket/object_key", ordenada lexicograficamente em BTreeMap
    objects: BTreeMap<String, ObjectManifest>,
}

impl MetadataStateMachine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Aplica um comando atômico no catálogo de metadados
    pub fn apply(&mut self, index: u64, term: u64, command: MetadataCommand) {
        match command {
            MetadataCommand::CreateBucket { bucket, created_at } => {
                self.buckets.insert(bucket, created_at);
            }
            MetadataCommand::DeleteBucket { bucket } => {
                self.buckets.remove(&bucket);
                // Remove todos os objetos associados ao bucket
                let prefix = format!("{}/", bucket);
                let keys_to_remove: Vec<String> = self
                    .objects
                    .range(prefix.clone()..)
                    .take_while(|(k, _)| k.starts_with(&prefix))
                    .map(|(k, _)| k.clone())
                    .collect();
                for k in keys_to_remove {
                    self.objects.remove(&k);
                }
            }
            MetadataCommand::PutObjectManifest { manifest } => {
                let key = format!("{}/{}", manifest.metadata.bucket.as_str(), manifest.metadata.key.as_str());
                self.objects.insert(key, manifest);
            }
            MetadataCommand::DeleteObjectManifest { bucket, key } => {
                let manifest_key = format!("{}/{}", bucket, key);
                self.objects.remove(&manifest_key);
            }
            MetadataCommand::RenameObjectManifest {
                src_bucket,
                src_key,
                dest_bucket,
                dest_key,
            } => {
                let src_manifest_key = format!("{}/{}", src_bucket, src_key);
                if let Some(mut manifest) = self.objects.remove(&src_manifest_key) {
                    if let Ok(b) = z3s_common::types::BucketName::new(dest_bucket.clone()) {
                        manifest.metadata.bucket = b;
                    }
                    if let Ok(k) = z3s_common::types::ObjectKey::new(dest_key.clone()) {
                        manifest.metadata.key = k;
                    }
                    let dest_manifest_key = format!("{}/{}", dest_bucket, dest_key);
                    self.objects.insert(dest_manifest_key, manifest);
                }
            }
        }

        self.last_applied_index = index;
        self.last_applied_term = term;
    }

    pub fn last_applied_index(&self) -> u64 {
        self.last_applied_index
    }

    pub fn last_applied_term(&self) -> u64 {
        self.last_applied_term
    }

    pub fn bucket_exists(&self, bucket: &str) -> bool {
        self.buckets.contains_key(bucket)
    }

    pub fn list_buckets(&self) -> Vec<(String, DateTime<Utc>)> {
        self.buckets.iter().map(|(k, v)| (k.clone(), *v)).collect()
    }

    pub fn get_object(&self, bucket: &str, key: &str) -> Option<&ObjectManifest> {
        let manifest_key = format!("{}/{}", bucket, key);
        self.objects.get(&manifest_key)
    }

    /// Listagem lexicográfica eficiente por prefixo utilizando BTreeMap range scan
    pub fn list_objects_range<'a>(
        &'a self,
        bucket: &str,
        prefix: &str,
    ) -> impl Iterator<Item = (&'a String, &'a ObjectManifest)> {
        let start_key = format!("{}/{}", bucket, prefix);
        let bucket_prefix = format!("{}/", bucket);
        self.objects
            .range(start_key.clone()..)
            .take_while(move |(k, _)| k.starts_with(&start_key) && k.starts_with(&bucket_prefix))
    }

    /// Retorna todos os manifestos de objetos registrados no catálogo
    pub fn list_all_manifests(&self) -> Vec<ObjectManifest> {
        self.objects.values().cloned().collect()
    }

    /// Cria um snapshot completo da máquina de estados
    pub fn create_snapshot(&self) -> MetadataSnapshot {
        MetadataSnapshot {
            last_applied_index: self.last_applied_index,
            last_applied_term: self.last_applied_term,
            buckets: self.buckets.clone(),
            objects: self.objects.clone(),
        }
    }

    /// Restaura o estado a partir de um snapshot
    pub fn restore_snapshot(&mut self, snapshot: MetadataSnapshot) {
        self.last_applied_index = snapshot.last_applied_index;
        self.last_applied_term = snapshot.last_applied_term;
        self.buckets = snapshot.buckets;
        self.objects = snapshot.objects;
    }
}
