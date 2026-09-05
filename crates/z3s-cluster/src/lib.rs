//! # z3s-cluster
//!
//! Coordenação de cluster, topologia de rede física, detecção de falhas e Placement Driver.

pub mod coordinator;
pub mod failure_detector;
pub mod node;
pub mod placement;
pub mod topology;

pub use coordinator::ClusterCoordinator;
pub use failure_detector::{FailureDetector, FailureDetectorConfig};
pub use node::{DiskInfo, NodeStatus, RackId, StorageNodeInfo, ZoneId};
pub use placement::{PlacementDriver, PlacementError};
pub use topology::TopologyMap;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use std::net::SocketAddr;
    use uuid::Uuid;
    use z3s_common::types::NodeId;

    #[test]
    fn test_topology_and_failure_domain_registration() {
        let mut topo = TopologyMap::new();

        let node1 = StorageNodeInfo::new(
            NodeId(Uuid::new_v4()),
            "10.0.1.1:9000".parse().unwrap(),
            ZoneId::new("us-east-1a"),
            RackId::new("rack-1"),
            1_000_000_000,
        );

        let node2 = StorageNodeInfo::new(
            NodeId(Uuid::new_v4()),
            "10.0.1.2:9000".parse().unwrap(),
            ZoneId::new("us-east-1a"),
            RackId::new("rack-2"),
            1_000_000_000,
        );

        let node3 = StorageNodeInfo::new(
            NodeId(Uuid::new_v4()),
            "10.0.2.1:9000".parse().unwrap(),
            ZoneId::new("us-east-1b"),
            RackId::new("rack-1"),
            1_000_000_000,
        );

        topo.register_node(node1.clone());
        topo.register_node(node2.clone());
        topo.register_node(node3.clone());

        assert_eq!(topo.total_nodes_count(), 3);
        assert_eq!(topo.active_nodes_count(), 3);
        assert_eq!(topo.zones().len(), 2);
        assert_eq!(topo.racks_in_zone(&ZoneId::new("us-east-1a")).len(), 2);
    }

    #[test]
    fn test_failure_detector_transitions() {
        let config = FailureDetectorConfig {
            suspect_timeout_secs: 2,
            dead_timeout_secs: 5,
            window_size: 10,
        };
        let mut fd = FailureDetector::new(config);
        let node_id = NodeId(Uuid::new_v4());
        let t0 = Utc::now();

        fd.record_heartbeat(node_id, t0);

        // 1 segundo depois -> Active
        assert_eq!(fd.evaluate_node(&node_id, t0 + Duration::seconds(1)), NodeStatus::Active);

        // 3 segundos depois -> Suspect
        assert_eq!(fd.evaluate_node(&node_id, t0 + Duration::seconds(3)), NodeStatus::Suspect);

        // 6 segundos depois -> Dead
        assert_eq!(fd.evaluate_node(&node_id, t0 + Duration::seconds(6)), NodeStatus::Dead);
    }

    #[test]
    fn test_placement_driver_failure_domain_isolation() {
        let mut topo = TopologyMap::new();

        // Cria 6 nós espalhados em 3 racks de 2 zonas
        let mut node_ids = Vec::new();
        for i in 0..6 {
            let zone = if i < 3 { ZoneId::new("zone-a") } else { ZoneId::new("zone-b") };
            let rack = RackId::new(format!("rack-{}", (i % 3) + 1));
            let addr: SocketAddr = format!("10.0.0.{}:9000", i + 1).parse().unwrap();
            let id = NodeId(Uuid::new_v4());
            node_ids.push(id);

            let node = StorageNodeInfo::new(id, addr, zone, rack, 2_000_000_000);
            topo.register_node(node);
        }

        // Seleciona 6 nós para um objeto K=4, M=2
        let placement = PlacementDriver::select_nodes_for_object(
            &topo,
            "meubucket",
            "videos/backup.mp4",
            6,
        ).unwrap();

        assert_eq!(placement.len(), 6);
        // Garante que todos os 6 nós selecionados são distintos
        let unique_nodes: std::collections::HashSet<NodeId> = placement.into_iter().collect();
        assert_eq!(unique_nodes.len(), 6);
    }
}
