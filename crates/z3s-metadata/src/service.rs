use crate::command::{MetadataCommand, MetadataLogEntry};
use crate::partition::PartitionRouter;
use crate::state_machine::MetadataStateMachine;
use crate::witness_cache::WitnessCache;
use chrono::{DateTime, Utc};
use std::sync::{Arc, RwLock};
use z3s_common::manifest::ObjectManifest;

/// Serviço Unificado de Metadados Distribuídos
pub struct DistributedMetadataService {
    current_term: RwLock<u64>,
    last_log_index: RwLock<u64>,
    state_machine: Arc<RwLock<MetadataStateMachine>>,
    router: Arc<RwLock<PartitionRouter>>,
    witness_cache: Arc<WitnessCache>,
    log: Arc<RwLock<Vec<MetadataLogEntry>>>,
}

impl Default for DistributedMetadataService {
    fn default() -> Self {
        Self {
            current_term: RwLock::new(1),
            last_log_index: RwLock::new(0),
            state_machine: Arc::new(RwLock::new(MetadataStateMachine::new())),
            router: Arc::new(RwLock::new(PartitionRouter::new())),
            witness_cache: Arc::new(WitnessCache::new()),
            log: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl DistributedMetadataService {
    pub fn new() -> Self {
        Self::default()
    }

    /// Submete e aplica um comando de metadados de forma linearizável
    pub fn commit_command(&self, command: MetadataCommand) -> Result<u64, &'static str> {
        let term = *self.current_term.read().unwrap();
        let mut index_guard = self.last_log_index.write().unwrap();
        *index_guard += 1;
        let index = *index_guard;

        let entry = MetadataLogEntry::new(index, term, command.clone());
        if !entry.is_valid() {
            return Err("Checksum do log de metadados inválido");
        }

        // 1. Grava no log de consenso
        self.log.write().unwrap().push(entry);

        // 2. Aplica na máquina de estados determinística
        let mut sm = self.state_machine.write().unwrap();
        sm.apply(index, term, command.clone());

        // 3. Atualiza o cache Witness
        match command {
            MetadataCommand::PutObjectManifest { manifest } => {
                let bucket = manifest.metadata.bucket.as_str().to_string();
                let key = manifest.metadata.key.as_str().to_string();
                self.witness_cache.put(
                    &bucket,
                    &key,
                    Some(manifest),
                    index,
                );
            }
            MetadataCommand::DeleteObjectManifest { bucket, key } => {
                self.witness_cache.put(&bucket, &key, None, index);
            }
            _ => {}
        }

        Ok(index)
    }

    /// Consulta objeto com validação do Witness Cache para latência ultra-baixa
    pub fn get_object(&self, bucket: &str, key: &str) -> Option<ObjectManifest> {
        let current_index = *self.last_log_index.read().unwrap();
        if let Some(cached) = self.witness_cache.get(bucket, key, current_index) {
            return cached;
        }

        let sm = self.state_machine.read().unwrap();
        let manifest = sm.get_object(bucket, key).cloned();
        self.witness_cache.put(bucket, key, manifest.clone(), current_index);
        manifest
    }

    pub fn list_buckets(&self) -> Vec<(String, DateTime<Utc>)> {
        self.state_machine.read().unwrap().list_buckets()
    }

    pub fn bucket_exists(&self, bucket: &str) -> bool {
        self.state_machine.read().unwrap().bucket_exists(bucket)
    }

    pub fn router(&self) -> Arc<RwLock<PartitionRouter>> {
        self.router.clone()
    }

    pub fn list_objects_range(&self, bucket: &str, prefix: &str) -> Vec<ObjectManifest> {
        let sm = self.state_machine.read().unwrap();
        sm.list_objects_range(bucket, prefix)
            .map(|(_, m)| m.clone())
            .collect()
    }
}
