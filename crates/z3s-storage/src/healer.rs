use crate::engine::{StorageEngine, StorageError};
use std::sync::Arc;
use thiserror::Error;
use z3s_common::manifest::{ObjectManifest, ShardPointer};
use z3s_erasure::{ErasureEngine, ErasureError};

#[derive(Debug, Error)]
pub enum HealingError {
    #[error("Parte/Manifesto irrecuperável (perda de fragmentos excedeu tolerância M): {0}")]
    Unrecoverable(String),

    #[error("Erro de codec de paridade Reed-Solomon: {0}")]
    Codec(#[from] ErasureError),

    #[error("Erro no Storage Engine: {0}")]
    Storage(#[from] StorageError),
}

/// Relatório consolidado da execução de Auto-Healing
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HealingReport {
    pub total_objects_scanned: usize,
    pub total_parts_scanned: usize,
    pub corrupted_or_missing_shards_detected: usize,
    pub shards_repaired_successfully: usize,
    pub unrecoverable_parts: usize,
}

/// Motor de Auto-Healing ativo (Auto-Cura de Bitrot & Perda de Shards)
pub struct AutoHealingEngine {
    engine: Arc<StorageEngine>,
    codec: ErasureEngine,
}

impl AutoHealingEngine {
    pub fn new(engine: Arc<StorageEngine>, data_shards: usize, parity_shards: usize) -> Result<Self, ErasureError> {
        Ok(Self {
            engine,
            codec: ErasureEngine::new(data_shards, parity_shards)?,
        })
    }

    /// Verifica e repara um grupo de shards (de uma parte ou objeto simples)
    pub fn heal_shards(
        &self,
        shards: &[ShardPointer],
        expected_total_size: u64,
    ) -> Result<usize, HealingError> {
        let total_shards = self.codec.total_shards();
        if shards.len() != total_shards {
            return Ok(0);
        }

        let mut shard_buffers: Vec<Option<Vec<u8>>> = vec![None; total_shards];
        let mut corrupted_indices = Vec::new();

        // 1. Tenta ler cada um dos K + M shards do disco e validar integridade
        for (i, shard_info) in shards.iter().enumerate() {
            match self.engine.read_shard(&shard_info.shard_id.0) {
                Ok(data) => {
                    if data.len() as u64 == shard_info.length {
                        shard_buffers[i] = Some(data);
                    } else {
                        corrupted_indices.push((i, shard_info.shard_id.0));
                    }
                }
                Err(_) => {
                    corrupted_indices.push((i, shard_info.shard_id.0));
                }
            }
        }

        // Se nenhum shard está corrompido ou faltando, nada a fazer
        if corrupted_indices.is_empty() {
            return Ok(0);
        }

        let healthy_count = shard_buffers.iter().filter(|s| s.is_some()).count();
        if healthy_count < self.codec.data_shards() {
            return Err(HealingError::Unrecoverable(format!(
                "Saudáveis: {} < Necessários: {}",
                healthy_count, self.codec.data_shards()
            )));
        }

        // 2. Reconstrução completa dos dados originais usando Reed-Solomon
        let original_data = self.codec.reconstruct(
            &mut shard_buffers,
            expected_total_size as usize,
        )?;

        // 3. Re-codifica para gerar todos os fragmentos íntegros
        let pristine = self.codec.encode(&original_data)?;
        let mut all_pristine = pristine.data_shards;
        all_pristine.extend(pristine.parity_shards);

        // 4. Grava de volta no StorageEngine os shards que estavam danificados/ausentes
        let mut repaired_count = 0;
        for (idx, shard_id) in corrupted_indices {
            let shard_payload = &all_pristine[idx];
            self.engine.write_shard(
                shard_id,
                idx as u32,
                total_shards as u32,
                shard_payload,
            )?;
            repaired_count += 1;
        }

        Ok(repaired_count)
    }

    /// Executa o Auto-Healing em todas as partes / shards de um manifesto de objeto
    pub fn heal_manifest(&self, manifest: &ObjectManifest) -> HealingReport {
        let mut report = HealingReport::default();
        report.total_objects_scanned += 1;

        if manifest.parts.is_empty() {
            // Objeto simples de parte única
            report.total_parts_scanned += 1;
            match self.heal_shards(&manifest.shards, manifest.metadata.size) {
                Ok(0) => {}
                Ok(repaired) => {
                    report.corrupted_or_missing_shards_detected += repaired;
                    report.shards_repaired_successfully += repaired;
                }
                Err(_) => {
                    report.unrecoverable_parts += 1;
                }
            }
        } else {
            // Objeto multipart com múltiplas partes
            for part in &manifest.parts {
                report.total_parts_scanned += 1;
                match self.heal_shards(&part.shards, part.size) {
                    Ok(0) => {}
                    Ok(repaired) => {
                        report.corrupted_or_missing_shards_detected += repaired;
                        report.shards_repaired_successfully += repaired;
                    }
                    Err(_) => {
                        report.unrecoverable_parts += 1;
                    }
                }
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::OpenOptions;
    use std::io::{Seek, SeekFrom, Write};
    use uuid::Uuid;
    use z3s_common::manifest::ObjectMetadata;
    use z3s_common::types::{BucketName, ETag, ObjectKey, ShardId, VersionId};
    use crate::extent::BLOCK_HEADER_SIZE;

    #[test]
    fn test_auto_healing_reconstructs_corrupted_and_missing_shards() {
        let temp_dir = tempfile::tempdir().unwrap();
        let engine = Arc::new(StorageEngine::open(temp_dir.path(), 10 * 1024 * 1024).unwrap());
        let healer = AutoHealingEngine::new(engine.clone(), 4, 2).unwrap();

        // 1. Cria um payload de teste e codifica em 4 dados + 2 paridade
        let original_data = b"Auto-Healing test payload verifying Reed-Solomon 4+2 resilient active reconstruction!".to_vec();
        let codec = ErasureEngine::new(4, 2).unwrap();
        let encoded = codec.encode(&original_data).unwrap();

        let mut all_shards_bytes = encoded.data_shards;
        all_shards_bytes.extend(encoded.parity_shards);

        let mut shard_pointers = Vec::new();
        let mut locations = Vec::new();

        for (i, shard_bytes) in all_shards_bytes.iter().enumerate() {
            let shard_id = Uuid::new_v4();
            let loc = engine.write_shard(shard_id, i as u32, 6, shard_bytes).unwrap();
            locations.push(loc);
            shard_pointers.push(ShardPointer {
                shard_id: ShardId(shard_id),
                shard_index: i as u32,
                is_parity: i >= 4,
                node_id: Uuid::new_v4(),
                extent_id: loc.extent_id,
                offset_in_extent: loc.offset_in_extent,
                length: shard_bytes.len() as u64,
                blake3_checksum: [0u8; 32],
            });
        }

        let metadata = ObjectMetadata {
            bucket: BucketName::new("test-bucket").unwrap(),
            key: ObjectKey::new("test-file.bin").unwrap(),
            version_id: VersionId::new(Uuid::now_v7().to_string()),
            size: original_data.len() as u64,
            etag: ETag::from_hex("0123456789abcdef0123456789abcdef"),
            content_type: "application/octet-stream".to_string(),
            storage_class: z3s_common::manifest::StorageClass::Standard,
            created_at: chrono::Utc::now(),
            user_metadata: Default::default(),
            merkle_root: [0u8; 32],
            is_delete_marker: false,
            is_latest: true,
            encryption: None,
        };

        let manifest = ObjectManifest::new(metadata, 4, 2, shard_pointers.clone());

        // 2. Corrompe o Shard 0 (dados) no disco e deleta o Shard 4 (paridade)
        let loc_0 = &locations[0];
        let extent_path = temp_dir.path().join("extents").join(format!("{}.z3se", loc_0.extent_id));
        let mut file = OpenOptions::new().read(true).write(true).open(&extent_path).unwrap();
        file.seek(SeekFrom::Start(loc_0.offset_in_extent + BLOCK_HEADER_SIZE as u64 + 1)).unwrap();
        file.write_all(b"BITROT_CORRUPTION").unwrap();
        file.flush().unwrap();

        // Deleta logicamente o shard de paridade 4
        engine.delete_shard(&shard_pointers[4].shard_id.0).unwrap();

        // 3. Executa Auto-Healing
        let report = healer.heal_manifest(&manifest);
        assert_eq!(report.corrupted_or_missing_shards_detected, 2);
        assert_eq!(report.shards_repaired_successfully, 2);
        assert_eq!(report.unrecoverable_parts, 0);

        // 4. Valida que todos os 6 shards agora estão legíveis e idênticos aos shards originais
        for (i, shard_ptr) in shard_pointers.iter().enumerate() {
            let read_shard = engine.read_shard(&shard_ptr.shard_id.0).unwrap();
            assert_eq!(read_shard, all_shards_bytes[i]);
        }
    }
}
