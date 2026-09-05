use chrono::{DateTime, Utc};
use std::collections::HashMap;
use z3s_common::types::NodeId;
use crate::node::NodeStatus;

/// Configuração do Detector de Falhas de Nós
#[derive(Debug, Clone)]
pub struct FailureDetectorConfig {
    /// Intervalo máximo tolerável sem heartbeat antes de marcar como Suspect (segundos)
    pub suspect_timeout_secs: i64,
    /// Intervalo máximo tolerável sem heartbeat antes de marcar como Dead (segundos)
    pub dead_timeout_secs: i64,
    /// Tamanho da janela de histórico de heartbeats
    pub window_size: usize,
}

impl Default for FailureDetectorConfig {
    fn default() -> Self {
        Self {
            suspect_timeout_secs: 5,
            dead_timeout_secs: 15,
            window_size: 100,
        }
    }
}

/// Registro dos últimos intervalos de heartbeat de um nó
#[derive(Debug, Clone)]
struct NodeHeartbeatHistory {
    last_seen: DateTime<Utc>,
    intervals_ms: Vec<u64>,
}

impl NodeHeartbeatHistory {
    fn new(now: DateTime<Utc>) -> Self {
        Self {
            last_seen: now,
            intervals_ms: Vec::new(),
        }
    }

    fn record_heartbeat(&mut self, now: DateTime<Utc>, max_window: usize) {
        let delta_ms = (now - self.last_seen).num_milliseconds();
        if delta_ms > 0 {
            self.intervals_ms.push(delta_ms as u64);
            if self.intervals_ms.len() > max_window {
                self.intervals_ms.remove(0);
            }
        }
        self.last_seen = now;
    }
}

/// Detector de Falhas de Nós baseado em Heartbeats e Janela Deslizante
#[derive(Debug, Clone)]
pub struct FailureDetector {
    config: FailureDetectorConfig,
    history: HashMap<NodeId, NodeHeartbeatHistory>,
}

impl FailureDetector {
    pub fn new(config: FailureDetectorConfig) -> Self {
        Self {
            config,
            history: HashMap::new(),
        }
    }

    /// Registra o recebimento de um heartbeat de um nó
    pub fn record_heartbeat(&mut self, node_id: NodeId, now: DateTime<Utc>) {
        let entry = self
            .history
            .entry(node_id)
            .or_insert_with(|| NodeHeartbeatHistory::new(now));
        entry.record_heartbeat(now, self.config.window_size);
    }

    /// Avalia o status de saúde de um nó baseado no tempo decorrido desde o último heartbeat
    pub fn evaluate_node(&self, node_id: &NodeId, now: DateTime<Utc>) -> NodeStatus {
        if let Some(hist) = self.history.get(node_id) {
            let elapsed = now - hist.last_seen;
            if elapsed.num_seconds() >= self.config.dead_timeout_secs {
                NodeStatus::Dead
            } else if elapsed.num_seconds() >= self.config.suspect_timeout_secs {
                NodeStatus::Suspect
            } else {
                NodeStatus::Active
            }
        } else {
            NodeStatus::Dead
        }
    }

    /// Remove o histórico de um nó removido do cluster
    pub fn forget_node(&mut self, node_id: &NodeId) {
        self.history.remove(node_id);
    }
}
