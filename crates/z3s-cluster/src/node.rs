use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;
use z3s_common::types::NodeId;

/// Identificador de Zona de Disponibilidade / Datacenter
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ZoneId(pub String);

impl ZoneId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl Default for ZoneId {
    fn default() -> Self {
        Self("zone-default".to_string())
    }
}

/// Identificador de Rack Físico dentro de uma zona
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RackId(pub String);

impl RackId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl Default for RackId {
    fn default() -> Self {
        Self("rack-default".to_string())
    }
}

/// Status de saúde e ciclo de vida de um nó no cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Nó ativo, respondendo a heartbeats e disponível para leitura e escrita
    Active,
    /// Heartbeat ausente momentaneamente (sob suspeita de falha)
    Suspect,
    /// Nó inalcançável e considerado indisponível
    Dead,
    /// Nó em processo de desativação (apenas leitura/migração de dados)
    Draining,
}

/// Informações de um disco físico anexado a um nó
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskInfo {
    pub disk_id: Uuid,
    pub path: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub is_healthy: bool,
}

/// Informações completas de um Storage Node registrado no cluster
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageNodeInfo {
    pub id: NodeId,
    pub address: SocketAddr,
    pub zone: ZoneId,
    pub rack: RackId,
    pub total_capacity_bytes: u64,
    pub used_capacity_bytes: u64,
    pub status: NodeStatus,
    pub last_heartbeat: DateTime<Utc>,
    pub disks: Vec<DiskInfo>,
}

impl StorageNodeInfo {
    pub fn new(
        id: NodeId,
        address: SocketAddr,
        zone: ZoneId,
        rack: RackId,
        total_capacity_bytes: u64,
    ) -> Self {
        Self {
            id,
            address,
            zone,
            rack,
            total_capacity_bytes,
            used_capacity_bytes: 0,
            status: NodeStatus::Active,
            last_heartbeat: Utc::now(),
            disks: Vec::new(),
        }
    }

    pub fn available_capacity_bytes(&self) -> u64 {
        self.total_capacity_bytes.saturating_sub(self.used_capacity_bytes)
    }

    pub fn is_available_for_writes(&self) -> bool {
        self.status == NodeStatus::Active && self.available_capacity_bytes() > 0
    }
}
