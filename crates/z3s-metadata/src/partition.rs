use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Identificador de uma partição de chaves do catálogo
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PartitionId(pub Uuid);

impl PartitionId {
    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Faixa de chaves gerenciada por uma partição: `[start_key, end_key)`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRange {
    /// Início inclusivo da faixa (vazio = início absoluto)
    pub start_key: String,
    /// Fim exclusivo da faixa (None = sem limite superior)
    pub end_key: Option<String>,
}

impl KeyRange {
    pub fn unbounded() -> Self {
        Self {
            start_key: String::new(),
            end_key: None,
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        if key < self.start_key.as_str() {
            return false;
        }
        if let Some(ref end) = self.end_key {
            if key >= end.as_str() {
                return false;
            }
        }
        true
    }
}

/// Informações de uma partição de catálogo de metadados
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataPartition {
    pub id: PartitionId,
    pub range: KeyRange,
    pub leader_node_id: Option<Uuid>,
    pub replica_node_ids: Vec<Uuid>,
}

/// Roteador de Partições Lexicográficas de Metadados
#[derive(Debug, Clone, Default)]
pub struct PartitionRouter {
    // start_key -> MetadataPartition
    partitions: BTreeMap<String, MetadataPartition>,
}

impl PartitionRouter {
    pub fn new() -> Self {
        let root_partition = MetadataPartition {
            id: PartitionId::new_v4(),
            range: KeyRange::unbounded(),
            leader_node_id: None,
            replica_node_ids: Vec::new(),
        };
        let mut router = Self::default();
        router.add_partition(root_partition);
        router
    }

    pub fn add_partition(&mut self, partition: MetadataPartition) {
        self.partitions.insert(partition.range.start_key.clone(), partition);
    }

    /// Localiza a partição responsável por uma determinada chave completa (`bucket/key`)
    pub fn route(&self, full_key: &str) -> Option<&MetadataPartition> {
        // Busca a maior start_key que seja <= full_key
        self.partitions
            .range(..=full_key.to_string())
            .next_back()
            .map(|(_, p)| p)
            .filter(|p| p.range.contains(full_key))
    }

    /// Retorna todas as partições ativas
    pub fn all_partitions(&self) -> Vec<&MetadataPartition> {
        self.partitions.values().collect()
    }
}
