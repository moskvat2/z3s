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

    /// Compacta todos os extents selados, descartando blocos de shards deletados e recuperando espaço em disco
    pub fn compact_sealed_extents(&self) -> Result<crate::compactor::CompactionReport, StorageError> {
        let mut report = crate::compactor::CompactionReport::default();
        let extents_dir = self.root_dir.join("extents");
        let active_id = {
            let active = self.active_extent.lock().unwrap();
            active.header.extent_id
        };

        let mut sealed_files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&extents_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("z3se") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(ext_id) = Uuid::parse_str(stem) {
                            if ext_id != active_id {
                                sealed_files.push((ext_id, path));
                            }
                        }
                    }
                }
            }
        }

        report.extents_scanned = sealed_files.len();

        for (ext_id, old_path) in sealed_files {
            let old_file_len = std::fs::metadata(&old_path).map(|m| m.len()).unwrap_or(0);

            // Coleta shards vivos pertencentes a este extent
            let all_locs = self.index.all_locations();
            let live_shards: Vec<ShardLocation> = all_locs
                .into_iter()
                .filter(|loc| loc.extent_id == ext_id)
                .collect();

            if live_shards.is_empty() {
                // Extent totalmente morto - remove direto do disco
                let mut sealed_map = self.sealed_extents.lock().unwrap();
                sealed_map.remove(&ext_id);
                drop(sealed_map);
                let _ = std::fs::remove_file(&old_path);
                report.extents_reclaimed += 1;
                report.bytes_reclaimed += old_file_len;
                continue;
            }

            // Compacta criando novo extent menor com apenas os shards vivos
            let live_bytes: u64 = live_shards.iter().map(|s| s.payload_length).sum();
            let new_extent_id = Uuid::new_v4();
            let new_path = extents_dir.join(format!("{}.z3se", new_extent_id));
            let required_capacity = (live_bytes + 64 * 1024).max(self.extent_capacity);
            let mut new_extent = ExtentFile::create(&new_path, new_extent_id, required_capacity)?;

            let mut relocated = Vec::new();
            for loc in &live_shards {
                let payload = self.read_shard(&loc.shard_id)?;
                let header = new_extent.append_shard(loc.shard_id, 0, 1, &payload)?;
                let new_loc = ShardLocation {
                    shard_id: loc.shard_id,
                    extent_id: new_extent_id,
                    offset_in_extent: header.offset_in_extent,
                    payload_length: header.payload_length,
                    checksum_blake3: header.checksum_blake3,
                };
                relocated.push(new_loc);
            }

            // Registra migrações no WAL e atualiza índice atômico
            let mut wal = self.wal.lock().unwrap();
            for new_loc in &relocated {
                wal.append(&WalRecord::ShardCommitted {
                    shard_id: new_loc.shard_id,
                    extent_id: new_loc.extent_id,
                    offset_in_extent: new_loc.offset_in_extent,
                    payload_length: new_loc.payload_length,
                    checksum_blake3: new_loc.checksum_blake3,
                })?;
                self.index.insert(new_loc.clone());
            }
            drop(wal);

            // Substitui no mapa de extents selados e deleta arquivo antigo
            let mut sealed_map = self.sealed_extents.lock().unwrap();
            sealed_map.remove(&ext_id);
            let opened_new = ExtentFile::open(&new_path)?;
            sealed_map.insert(new_extent_id, opened_new);
            drop(sealed_map);

            let new_file_len = std::fs::metadata(&new_path).map(|m| m.len()).unwrap_or(0);
            let _ = std::fs::remove_file(&old_path);

            report.extents_reclaimed += 1;
            report.live_shards_retained += live_shards.len();
            report.bytes_reclaimed += old_file_len.saturating_sub(new_file_len);
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_extent_compaction_reclaims_space() {
        let temp_dir = tempfile::tempdir().unwrap();
        // Cria engine com capacidade pequena de extent para forçar criação de extents selados
        let engine = Arc::new(StorageEngine::open(temp_dir.path(), 8192).unwrap());

        let shard_1 = Uuid::new_v4();
        let shard_2 = Uuid::new_v4();
        let shard_3 = Uuid::new_v4();

        // 1. Grava shards
        let payload = vec![0xAA; 3000];
        engine.write_shard(shard_1, 0, 3, &payload).unwrap();
        engine.write_shard(shard_2, 1, 3, &payload).unwrap();
        // Shard 3 vai estourar a capacidade do extent 1 e criar novo extent
        engine.write_shard(shard_3, 2, 3, &payload).unwrap();

        // 2. Deleta shard_1 (deixando buraco no primeiro extent)
        engine.delete_shard(&shard_1).unwrap();

        // 3. Roda a compactação
        let compactor = crate::compactor::ExtentCompactor::new(engine.clone());
        let report = compactor.run_compaction().unwrap();

        assert!(report.extents_reclaimed > 0);
        assert_eq!(report.live_shards_retained, 1); // shard_2 foi mantido e relocado

        // 4. Verifica que os shards vivos continuam legíveis com conteúdo intacto
        let read_2 = engine.read_shard(&shard_2).unwrap();
        assert_eq!(read_2, payload);
        let read_3 = engine.read_shard(&shard_3).unwrap();
        assert_eq!(read_3, payload);

        // shard_1 deve continuar reportado como não encontrado
        assert!(engine.read_shard(&shard_1).is_err());
    }

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
