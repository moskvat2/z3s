use crate::failure_detector::{FailureDetector, FailureDetectorConfig};
use crate::node::{NodeStatus, StorageNodeInfo};
use crate::placement::{PlacementDriver, PlacementError};
use crate::topology::TopologyMap;
use chrono::Utc;
use std::sync::{Arc, RwLock};
use z3s_common::types::NodeId;

/// Coordenador Central do Cluster Z3S
///
/// Mantém o mapa de topologia, processa batimentos cardíacos (heartbeats),
/// avalia a saúde dos nós e executa alocação de shards via Placement Driver.
pub struct ClusterCoordinator {
    topology: Arc<RwLock<TopologyMap>>,
    failure_detector: Arc<RwLock<FailureDetector>>,
}

impl ClusterCoordinator {
    pub fn new(config: FailureDetectorConfig) -> Self {
        Self {
            topology: Arc::new(RwLock::new(TopologyMap::new())),
            failure_detector: Arc::new(RwLock::new(FailureDetector::new(config))),
        }
    }

    /// Registra um novo nó no cluster
    pub fn register_node(&self, node: StorageNodeInfo) {
        let mut fd = self.failure_detector.write().unwrap();
        fd.record_heartbeat(node.id, Utc::now());

        let mut topo = self.topology.write().unwrap();
        topo.register_node(node);
    }

    /// Processa o recebimento de heartbeat de um nó
    pub fn handle_heartbeat(&self, node_id: NodeId) {
        let now = Utc::now();
        let mut fd = self.failure_detector.write().unwrap();
        fd.record_heartbeat(node_id, now);

        let mut topo = self.topology.write().unwrap();
        if let Some(node) = topo.get_node(&node_id) {
            if node.status == NodeStatus::Suspect || node.status == NodeStatus::Dead {
                topo.update_node_status(&node_id, NodeStatus::Active);
            }
        }
    }

    /// Executa ciclo periódico de verificação de saúde de todos os nós
    pub fn health_check_tick(&self) {
        let now = Utc::now();
        let fd = self.failure_detector.read().unwrap();
        let mut topo = self.topology.write().unwrap();

        let node_ids: Vec<NodeId> = topo.all_nodes().iter().map(|n| n.id).collect();
        for node_id in node_ids {
            let status = fd.evaluate_node(&node_id, now);
            topo.update_node_status(&node_id, status);
        }
    }

    /// Seleciona nós físicos para os shards de um objeto
    pub fn select_placement(
        &self,
        bucket: &str,
        key: &str,
        shard_count: usize,
    ) -> Result<Vec<NodeId>, PlacementError> {
        let topo = self.topology.read().unwrap();
        PlacementDriver::select_nodes_for_object(&topo, bucket, key, shard_count)
    }

    /// Snapshot da topologia do cluster
    pub fn topology_snapshot(&self) -> TopologyMap {
        self.topology.read().unwrap().clone()
    }
}
