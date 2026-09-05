use crate::engine::StorageEngine;
use std::sync::Arc;
use uuid::Uuid;

/// Relatório de integridade produzido pelo Scrubber de Bitrot
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScrubReport {
    pub total_shards_checked: usize,
    pub healthy_shards: usize,
    pub corrupted_shards: Vec<Uuid>,
}

/// Scrubber de Bitrot que verifica a integridade de todos os blocos em repouso no disco
pub struct BitrotScrubber {
    engine: Arc<StorageEngine>,
}

impl BitrotScrubber {
    pub fn new(engine: Arc<StorageEngine>) -> Self {
        Self { engine }
    }

    /// Executa uma varredura completa em todos os shards registrados no nó
    pub fn scrub_all(&self) -> ScrubReport {
        let locations = self.engine.list_shards();
        let mut report = ScrubReport {
            total_shards_checked: locations.len(),
            healthy_shards: 0,
            corrupted_shards: Vec::new(),
        };

        for location in locations {
            match self.engine.read_shard(&location.shard_id) {
                Ok(_) => {
                    report.healthy_shards += 1;
                }
                Err(_) => {
                    report.corrupted_shards.push(location.shard_id);
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
    use crate::extent::BLOCK_HEADER_SIZE;

    #[test]
    fn test_bitrot_scrubber_detects_corrupted_shards() {
        let temp_dir = tempfile::tempdir().unwrap();
        let engine = Arc::new(StorageEngine::open(temp_dir.path(), 10 * 1024 * 1024).unwrap());

        // 1. Grava 3 shards saudáveis
        let shard_1 = Uuid::new_v4();
        let shard_2 = Uuid::new_v4();
        let shard_3 = Uuid::new_v4();

        let loc_1 = engine.write_shard(shard_1, 0, 3, b"Shard 1 saudavel").unwrap();
        engine.write_shard(shard_2, 1, 3, b"Shard 2 saudavel").unwrap();
        engine.write_shard(shard_3, 2, 3, b"Shard 3 saudavel").unwrap();

        // 2. Executa scrub -> Todos 3 devem estar saudáveis
        let scrubber = BitrotScrubber::new(engine.clone());
        let report_initial = scrubber.scrub_all();
        assert_eq!(report_initial.total_shards_checked, 3);
        assert_eq!(report_initial.healthy_shards, 3);
        assert!(report_initial.corrupted_shards.is_empty());

        // 3. Corrompe o payload do Shard 1 fisicamente no arquivo de extent
        let extent_path = temp_dir.path().join("extents").join(format!("{}.z3se", loc_1.extent_id));
        let mut file = OpenOptions::new().read(true).write(true).open(&extent_path).unwrap();
        file.seek(SeekFrom::Start(loc_1.offset_in_extent + BLOCK_HEADER_SIZE as u64 + 2)).unwrap();
        file.write_all(b"CORRUPTED").unwrap();
        file.flush().unwrap();

        // 4. Executa scrub novamente -> Deve acusar Shard 1 como corrompido
        let report_after_bitrot = scrubber.scrub_all();
        assert_eq!(report_after_bitrot.total_shards_checked, 3);
        assert_eq!(report_after_bitrot.healthy_shards, 2);
        assert_eq!(report_after_bitrot.corrupted_shards, vec![shard_1]);
    }
}
