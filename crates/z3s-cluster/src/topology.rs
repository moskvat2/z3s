use crate::node::{NodeStatus, RackId, StorageNodeInfo, ZoneId};
use std::collections::{BTreeMap, HashMap, HashSet};
use z3s_common::types::NodeId;

/// Mapeamento da Topologia Física do Cluster Z3S
#[derive(Debug, Clone, Default)]
pub struct TopologyMap {
    nodes: HashMap<NodeId, StorageNodeInfo>,
    // zone -> rack -> set(node_id)
    zones: BTreeMap<ZoneId, BTreeMap<RackId, HashSet<NodeId>>>,
}

impl TopologyMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra ou atualiza um nó na topologia
    pub fn register_node(&mut self, node: StorageNodeInfo) {
        let node_id = node.id;
        let zone = node.zone.clone();
        let rack = node.rack.clone();

        // Se já existia em outra zona/rack, remove da hierarquia antiga
        if let Some(existing) = self.nodes.get(&node_id) {
            if existing.zone != zone || existing.rack != rack {
                if let Some(racks) = self.zones.get_mut(&existing.zone) {
                    if let Some(nodes) = racks.get_mut(&existing.rack) {
                        nodes.remove(&node_id);
                    }
                }
            }
        }

        self.zones
            .entry(zone)
            .or_default()
            .entry(rack)
            .or_default()
            .insert(node_id);

        self.nodes.insert(node_id, node);
    }

    /// Remove um nó da topologia
    pub fn unregister_node(&mut self, node_id: &NodeId) -> Option<StorageNodeInfo> {
        if let Some(removed) = self.nodes.remove(node_id) {
            if let Some(racks) = self.zones.get_mut(&removed.zone) {
                if let Some(nodes) = racks.get_mut(&removed.rack) {
                    nodes.remove(node_id);
                }
            }
            Some(removed)
        } else {
            None
        }
    }

    /// Atualiza o status de um nó
    pub fn update_node_status(&mut self, node_id: &NodeId, status: NodeStatus) -> bool {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.status = status;
            true
        } else {
            false
        }
    }

    /// Obtém informações de um nó por ID
    pub fn get_node(&self, node_id: &NodeId) -> Option<&StorageNodeInfo> {
        self.nodes.get(node_id)
    }

    /// Retorna todos os nós do cluster
    pub fn all_nodes(&self) -> Vec<&StorageNodeInfo> {
        self.nodes.values().collect()
    }

    /// Retorna todos os nós ativos e disponíveis para escrita
    pub fn active_nodes(&self) -> Vec<&StorageNodeInfo> {
        self.nodes
            .values()
            .filter(|n| n.is_available_for_writes())
            .collect()
    }

    /// Contagem total de nós
    pub fn total_nodes_count(&self) -> usize {
        self.nodes.len()
    }

    /// Contagem de nós ativos
    pub fn active_nodes_count(&self) -> usize {
        self.nodes.values().filter(|n| n.status == NodeStatus::Active).count()
    }

    /// Total de capacidade física (bytes)
    pub fn total_capacity_bytes(&self) -> u64 {
        self.nodes.values().map(|n| n.total_capacity_bytes).sum()
    }

    /// Total de capacidade usada (bytes)
    pub fn used_capacity_bytes(&self) -> u64 {
        self.nodes.values().map(|n| n.used_capacity_bytes).sum()
    }

    /// Lista todas as zonas registradas
    pub fn zones(&self) -> Vec<&ZoneId> {
        self.zones.keys().collect()
    }

    /// Lista todos os racks em uma determinada zona
    pub fn racks_in_zone(&self, zone: &ZoneId) -> Vec<&RackId> {
        self.zones
            .get(zone)
            .map(|r| r.keys().collect())
            .unwrap_or_default()
    }
}
