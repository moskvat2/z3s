use crate::engine::{StorageEngine, StorageError};
use std::sync::Arc;

/// Relatório de compactação de extents
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompactionReport {
    pub extents_scanned: usize,
    pub extents_reclaimed: usize,
    pub live_shards_retained: usize,
    pub dead_shards_purged: usize,
    pub bytes_reclaimed: u64,
}

/// Compactador de arquivos de Extent em repouso
pub struct ExtentCompactor {
    engine: Arc<StorageEngine>,
}

impl ExtentCompactor {
    pub fn new(engine: Arc<StorageEngine>) -> Self {
        Self { engine }
    }

    /// Executa a compactação de todos os extents selados que possuem espaço desperdiçado
    pub fn run_compaction(&self) -> Result<CompactionReport, StorageError> {
        self.engine.compact_sealed_extents()
    }
}
