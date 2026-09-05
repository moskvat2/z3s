use crate::extent::{ExtentError, ExtentFile};
use crate::index::{ShardLocation, StorageIndex};
use crate::wal::{WalError, WalRecord, WriteAheadLog};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

/// Tamanho padrão de um arquivo Extent (64 MB)
pub const DEFAULT_EXTENT_CAPACITY: u64 = 64 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Shard não encontrado: {0}")]
    ShardNotFound(Uuid),

    #[error("Erro de Extent no disco: {0}")]
    Extent(#[from] ExtentError),

    #[error("Erro no Write-Ahead Log: {0}")]
    Wal(#[from] WalError),

    #[error("Erro de I/O de armazenamento: {0}")]
    Io(#[from] std::io::Error),

    #[error("Intervalo de bytes inválido: offset {offset} + len {length} > tamanho total {total}")]
    InvalidRange { offset: u64, length: u64, total: u64 },
}

/// Motor Central de Armazenamento Local (Storage Engine)
pub struct StorageEngine {
    root_dir: PathBuf,
    extent_capacity: u64,
    index: Arc<StorageIndex>,
    wal: Mutex<WriteAheadLog>,
    active_extent: Mutex<ExtentFile>,
    sealed_extents: Mutex<HashMap<Uuid, ExtentFile>>,
}

impl StorageEngine {
    /// Inicializa o Storage Engine no diretório raiz especificado e recupera o estado via WAL
    pub fn open(root_dir: impl AsRef<Path>, extent_capacity: u64) -> Result<Self, StorageError> {
        let root_dir = root_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&root_dir)?;

        let extents_dir = root_dir.join("extents");
        std::fs::create_dir_all(&extents_dir)?;

        let wal_path = root_dir.join("storage.wal");
        let wal = WriteAheadLog::open(&wal_path)?;
        let index = Arc::new(StorageIndex::new());

        // Replay do WAL para restaurar índice em memória
        let replayed_records = WriteAheadLog::replay(&wal_path)?;
        for record in replayed_records {
            match record {
                WalRecord::ShardCommitted {
                    shard_id,
                    extent_id,
                    offset_in_extent,
                    payload_length,
                    checksum_blake3,
                } => {
                    index.insert(ShardLocation {
                        shard_id,
                        extent_id,
                        offset_in_extent,
                        payload_length,
                        checksum_blake3,
                    });
                }
                WalRecord::ShardDeleted { shard_id } => {
                    index.remove(&shard_id);
                }
            }
        }

        // Inicializa ou abre o extent ativo
        let active_extent_id = Uuid::new_v4();
        let active_extent_path = extents_dir.join(format!("{}.z3se", active_extent_id));
        let active_extent = ExtentFile::create(&active_extent_path, active_extent_id, extent_capacity)?;

        Ok(Self {
            root_dir,
            extent_capacity,
            index,
            wal: Mutex::new(wal),
            active_extent: Mutex::new(active_extent),
            sealed_extents: Mutex::new(HashMap::new()),
        })
    }

    /// Grava um Shard atomicamente com registro no WAL e no Extent ativo
    pub fn write_shard(
        &self,
        shard_id: Uuid,
        shard_index: u32,
        total_shards: u32,
        payload: &[u8],
    ) -> Result<ShardLocation, StorageError> {
        let mut active_extent = self.active_extent.lock().unwrap();

        // Se o extent atual estiver cheio, sela-o e cria um novo
        let block_header = match active_extent.append_shard(shard_id, shard_index, total_shards, payload) {
            Ok(header) => header,
            Err(ExtentError::ExtentFull) => {
                let sealed_id = active_extent.header.extent_id;
                let sealed_path = active_extent.path.clone();
                let opened_sealed = ExtentFile::open(&sealed_path)?;

                let mut sealed_map = self.sealed_extents.lock().unwrap();
                sealed_map.insert(sealed_id, opened_sealed);

                // Cria novo extent ativo garantindo capacidade suficiente para o payload
                let required_capacity = (payload.len() as u64 + 8192).max(self.extent_capacity);
                let new_extent_id = Uuid::new_v4();
                let new_extent_path = self.root_dir.join("extents").join(format!("{}.z3se", new_extent_id));
                *active_extent = ExtentFile::create(&new_extent_path, new_extent_id, required_capacity)?;

                active_extent.append_shard(shard_id, shard_index, total_shards, payload)?
            }
            Err(e) => return Err(StorageError::Extent(e)),
        };

        let location = ShardLocation {
            shard_id,
            extent_id: active_extent.header.extent_id,
            offset_in_extent: block_header.offset_in_extent,
            payload_length: block_header.payload_length,
            checksum_blake3: block_header.checksum_blake3,
        };

        // Grava no WAL para garantir crash-consistency
        let mut wal = self.wal.lock().unwrap();
        wal.append(&WalRecord::ShardCommitted {
            shard_id,
            extent_id: location.extent_id,
            offset_in_extent: location.offset_in_extent,
            payload_length: location.payload_length,
            checksum_blake3: location.checksum_blake3,
        })?;

        // Atualiza índice em memória
        self.index.insert(location);

        Ok(location)
    }

    /// Lê o payload completo de um Shard e valida sua integridade via BLAKE3 on-the-fly
    pub fn read_shard(&self, shard_id: &Uuid) -> Result<Vec<u8>, StorageError> {
        let location = self
            .index
            .get(shard_id)
            .ok_or(StorageError::ShardNotFound(*shard_id))?;

        let mut active_extent = self.active_extent.lock().unwrap();
        if active_extent.header.extent_id == location.extent_id {
            let (_, payload) = active_extent.read_shard(location.offset_in_extent)?;
            return Ok(payload);
        }
        drop(active_extent);

        let mut sealed = self.sealed_extents.lock().unwrap();
        if let Some(extent) = sealed.get_mut(&location.extent_id) {
            let (_, payload) = extent.read_shard(location.offset_in_extent)?;
            return Ok(payload);
        }

        // Se o arquivo estiver fechado no pool, abre e lê
        let extent_path = self
            .root_dir
            .join("extents")
            .join(format!("{}.z3se", location.extent_id));
        let mut extent = ExtentFile::open(&extent_path)?;
        let (_, payload) = extent.read_shard(location.offset_in_extent)?;
        sealed.insert(location.extent_id, extent);

        Ok(payload)
    }

    /// Lê uma faixa específica de bytes (Byte-Range) de um Shard
    pub fn read_shard_range(
        &self,
        shard_id: &Uuid,
        offset: u64,
        length: u64,
    ) -> Result<Vec<u8>, StorageError> {
        let full_payload = self.read_shard(shard_id)?;
        let total_len = full_payload.len() as u64;

        if offset + length > total_len {
            return Err(StorageError::InvalidRange {
                offset,
                length,
                total: total_len,
            });
        }

        let start = offset as usize;
        let end = (offset + length) as usize;
        Ok(full_payload[start..end].to_vec())
    }

    /// Deleta logicamente um Shard e registra no WAL
    pub fn delete_shard(&self, shard_id: &Uuid) -> Result<(), StorageError> {
        if self.index.remove(shard_id).is_some() {
            let mut wal = self.wal.lock().unwrap();
            wal.append(&WalRecord::ShardDeleted { shard_id: *shard_id })?;
        }
        Ok(())
    }

    /// Retorna a lista de todos os shards indexados
    pub fn list_shards(&self) -> Vec<ShardLocation> {
        self.index.all_locations()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_engine_write_read_range_delete() {
        let temp_dir = tempfile::tempdir().unwrap();
        let engine = StorageEngine::open(temp_dir.path(), 10 * 1024 * 1024).unwrap();

        let shard_id = Uuid::new_v4();
        let payload = b"0123456789ABCDEF-Z3S-STORAGE-TEST-DATA";

        // 1. Grava Shard
        let location = engine.write_shard(shard_id, 0, 1, payload).unwrap();
        assert_eq!(location.shard_id, shard_id);

        // 2. Lê Shard completo
        let read_data = engine.read_shard(&shard_id).unwrap();
        assert_eq!(read_data, payload);

        // 3. Lê Byte-Range (ex: bytes 10 a 16)
        let range_data = engine.read_shard_range(&shard_id, 10, 6).unwrap();
        assert_eq!(range_data, b"ABCDEF");

        // 4. Deleta Shard
        engine.delete_shard(&shard_id).unwrap();
        assert!(matches!(
            engine.read_shard(&shard_id),
            Err(StorageError::ShardNotFound(_))
        ));
    }

    #[test]
    fn test_storage_engine_crash_recovery_via_wal() {
        let temp_dir = tempfile::tempdir().unwrap();
        let shard_id = Uuid::new_v4();
        let payload = b"Payload persistente para teste de recuperacao pos-queda de energia";

        {
            // Instancia o engine, grava o dado e força o encerramento (drop)
            let engine = StorageEngine::open(temp_dir.path(), 10 * 1024 * 1024).unwrap();
            engine.write_shard(shard_id, 0, 1, payload).unwrap();
        }

        // Reabre o StorageEngine a partir do mesmo diretório -> deve reconstruir o índice via WAL
        let recovered_engine = StorageEngine::open(temp_dir.path(), 10 * 1024 * 1024).unwrap();
        let read_data = recovered_engine.read_shard(&shard_id).unwrap();
        assert_eq!(read_data, payload);
    }
}
