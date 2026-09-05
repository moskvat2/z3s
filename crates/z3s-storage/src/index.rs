use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

/// Localização física exata de um Shard dentro de um arquivo Extent
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShardLocation {
    pub shard_id: Uuid,
    pub extent_id: Uuid,
    pub offset_in_extent: u64,
    pub payload_length: u64,
    pub checksum_blake3: [u8; 32],
}

/// Índice em memória thread-safe para localização ultra-rápida de shards (O(1))
pub struct StorageIndex {
    entries: RwLock<HashMap<Uuid, ShardLocation>>,
}

impl StorageIndex {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Insere ou atualiza a localização de um Shard
    pub fn insert(&self, location: ShardLocation) {
        let mut map = self.entries.write().unwrap();
        map.insert(location.shard_id, location);
    }

    /// Busca a localização de um Shard por seu ID
    pub fn get(&self, shard_id: &Uuid) -> Option<ShardLocation> {
        let map = self.entries.read().unwrap();
        map.get(shard_id).copied()
    }

    /// Remove a entrada de um Shard
    pub fn remove(&self, shard_id: &Uuid) -> Option<ShardLocation> {
        let mut map = self.entries.write().unwrap();
        map.remove(shard_id)
    }

    /// Retorna o total de shards indexados
    pub fn count(&self) -> usize {
        let map = self.entries.read().unwrap();
        map.len()
    }

    /// Retorna todos os shards indexados
    pub fn all_locations(&self) -> Vec<ShardLocation> {
        let map = self.entries.read().unwrap();
        map.values().copied().collect()
    }
}

impl Default for StorageIndex {
    fn default() -> Self {
        Self::new()
    }
}
