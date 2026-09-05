use crate::node::{RackId, ZoneId};
use crate::topology::TopologyMap;
use std::collections::HashSet;
use z3s_common::types::NodeId;

/// Algoritmo de Posicionamento de Shards (Placement Driver)
///
/// Responsável por selecionar nós de armazenamento para os K+M fragmentos de cada objeto,
/// garantindo distribuição uniforme (Rendezvous Hashing) e isolamento de domínios de falha (Racks/Zonas).
pub struct PlacementDriver;

impl PlacementDriver {
    /// Calcula o score de afinidade de um nó para uma determinada chave e índice de shard
    fn compute_node_score(key: &str, node_id: &NodeId, shard_index: usize) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(key.as_bytes());
        hasher.update(node_id.0.as_bytes());
        hasher.update(&(shard_index as u64).to_le_bytes());
        *hasher.finalize().as_bytes()
    }

    /// Seleciona `target_count` nós distintos para alocação dos shards de um objeto
    ///
    /// Prioriza a dispersão em zonas e racks diferentes para garantir tolerância máxima a desastres.
    pub fn select_nodes_for_object(
        topology: &TopologyMap,
        bucket: &str,
        key: &str,
        target_count: usize,
    ) -> Result<Vec<NodeId>, PlacementError> {
        let active_nodes = topology.active_nodes();
        if active_nodes.is_empty() {
            return Err(PlacementError::NoActiveNodes);
        }

        if active_nodes.len() < target_count {
            return Err(PlacementError::InsufficientNodes {
                available: active_nodes.len(),
                required: target_count,
            });
        }

        let full_key = format!("{}/{}", bucket, key);
        let mut selected_node_ids = Vec::with_capacity(target_count);
        let mut selected_racks = HashSet::new();
        let mut selected_zones = HashSet::new();
        let mut used_node_ids = HashSet::new();

        for shard_idx in 0..target_count {
            // Calcula scores para todos os nós disponíveis ainda não selecionados
            let mut candidates: Vec<(NodeId, &RackId, &ZoneId, [u8; 32])> = active_nodes
                .iter()
                .filter(|n| !used_node_ids.contains(&n.id))
                .map(|n| {
                    let score = Self::compute_node_score(&full_key, &n.id, shard_idx);
                    (n.id, &n.rack, &n.zone, score)
                })
                .collect();

            // Ordena os candidatos por pontuação determinística decrescente
            candidates.sort_by(|a, b| b.3.cmp(&a.3));

            // Passo 1: Tenta encontrar um candidato com Zona E Rack novos
            let best_pick = candidates
                .iter()
                .find(|(_, rack, zone, _)| !selected_zones.contains(*zone) && !selected_racks.contains(*rack))
                // Passo 2: Se não houver zona nova, tenta rack novo
                .or_else(|| {
                    candidates
                        .iter()
                        .find(|(_, rack, _, _)| !selected_racks.contains(*rack))
                })
                // Passo 3: Se todos os racks já estiverem em uso, pega o de maior score
                .or_else(|| candidates.first());

            if let Some((node_id, rack, zone, _)) = best_pick {
                selected_node_ids.push(*node_id);
                selected_racks.insert((*rack).clone());
                selected_zones.insert((*zone).clone());
                used_node_ids.insert(*node_id);
            } else {
                return Err(PlacementError::PlacementFailed);
            }
        }

        Ok(selected_node_ids)
    }
}

/// Erros na alocação de nós pelo Placement Driver
#[derive(Debug, thiserror::Error)]
pub enum PlacementError {
    #[error("Nenhum nó ativo disponível no cluster")]
    NoActiveNodes,

    #[error("Nós insuficientes no cluster: disponíveis={available}, necessários={required}")]
    InsufficientNodes { available: usize, required: usize },

    #[error("Falha ao alocar nós de acordo com a política de domínios de falha")]
    PlacementFailed,
}
