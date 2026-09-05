use crate::hash::blake3_hash;
use serde::{Deserialize, Serialize};

/// Árvore de Merkle binária otimizada para validação de chunks de objetos S3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleTree {
    /// Nós de todos os níveis da árvore (nível 0 = folhas, último nível = raiz)
    levels: Vec<Vec<[u8; 32]>>,
}

/// Prova de inclusão de Merkle para verificação independente de um chunk (ex: Byte-Range)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleProof {
    pub leaf_index: usize,
    pub total_leaves: usize,
    pub path: Vec<([u8; 32], bool)>, // (sibling_hash, is_sibling_left)
}

impl MerkleTree {
    /// Constrói uma árvore de Merkle a partir de uma lista de hashes de chunks (folhas)
    pub fn from_leaf_hashes(leaves: Vec<[u8; 32]>) -> Self {
        if leaves.is_empty() {
            return Self {
                levels: vec![vec![[0u8; 32]]],
            };
        }

        let mut levels = Vec::new();
        levels.push(leaves.clone());

        let mut current_level = leaves;
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in current_level.chunks(2) {
                if chunk.len() == 2 {
                    let combined = Self::hash_nodes(&chunk[0], &chunk[1]);
                    next_level.push(combined);
                } else {
                    // Se o número de nós for ímpar, duplica o último nó
                    let combined = Self::hash_nodes(&chunk[0], &chunk[0]);
                    next_level.push(combined);
                }
            }
            levels.push(next_level.clone());
            current_level = next_level;
        }

        Self { levels }
    }

    /// Constrói a árvore de Merkle a partir dos chunks brutos de dados
    pub fn from_data_chunks(chunks: &[&[u8]]) -> Self {
        let leaves: Vec<[u8; 32]> = chunks.iter().map(|c| blake3_hash(c)).collect();
        Self::from_leaf_hashes(leaves)
    }

    /// Retorna o Hash Raiz (Root Hash) da árvore
    pub fn root(&self) -> [u8; 32] {
        *self.levels.last().unwrap().first().unwrap()
    }

    /// Retorna a quantidade de folhas (chunks de dados)
    pub fn leaf_count(&self) -> usize {
        self.levels[0].len()
    }

    /// Gera uma prova de inclusão para uma folha específica
    pub fn generate_proof(&self, leaf_index: usize) -> Option<MerkleProof> {
        let total_leaves = self.leaf_count();
        if leaf_index >= total_leaves {
            return None;
        }

        let mut path = Vec::new();
        let mut index = leaf_index;

        for level in &self.levels[..self.levels.len() - 1] {
            let is_right_child = index % 2 == 1;
            let sibling_index = if is_right_child {
                index - 1
            } else if index + 1 < level.len() {
                index + 1
            } else {
                index // Duplicado se for ímpar no fim
            };

            let sibling_hash = level[sibling_index];
            let is_sibling_left = is_right_child;
            path.push((sibling_hash, is_sibling_left));
            index /= 2;
        }

        Some(MerkleProof {
            leaf_index,
            total_leaves,
            path,
        })
    }

    /// Valida se um hash de folha pertence à raiz da árvore através da prova
    pub fn verify_proof(root: &[u8; 32], leaf_hash: &[u8; 32], proof: &MerkleProof) -> bool {
        let mut current_hash = *leaf_hash;

        for (sibling_hash, is_sibling_left) in &proof.path {
            if *is_sibling_left {
                current_hash = Self::hash_nodes(sibling_hash, &current_hash);
            } else {
                current_hash = Self::hash_nodes(&current_hash, sibling_hash);
            }
        }

        &current_hash == root
    }

    fn hash_nodes(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(left);
        combined[32..].copy_from_slice(right);
        blake3_hash(&combined)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree_root_and_verification() {
        let chunks: Vec<&[u8]> = vec![
            b"chunk-0-payload-data",
            b"chunk-1-payload-data",
            b"chunk-2-payload-data",
            b"chunk-3-payload-data",
        ];

        let tree = MerkleTree::from_data_chunks(&chunks);
        let root = tree.root();

        // Valida cada um dos chunks com suas provas individuais
        for (i, chunk) in chunks.iter().enumerate() {
            let proof = tree.generate_proof(i).unwrap();
            let leaf_hash = blake3_hash(chunk);
            assert!(MerkleTree::verify_proof(&root, &leaf_hash, &proof));
        }
    }

    #[test]
    fn test_merkle_tree_odd_number_of_chunks() {
        let chunks: Vec<&[u8]> = vec![
            b"chunk-A",
            b"chunk-B",
            b"chunk-C", // 3 chunks (ímpar)
        ];

        let tree = MerkleTree::from_data_chunks(&chunks);
        let root = tree.root();

        for (i, chunk) in chunks.iter().enumerate() {
            let proof = tree.generate_proof(i).unwrap();
            let leaf_hash = blake3_hash(chunk);
            assert!(MerkleTree::verify_proof(&root, &leaf_hash, &proof));
        }
    }

    #[test]
    fn test_merkle_tree_detects_tampered_chunk() {
        let chunks: Vec<&[u8]> = vec![b"dados-1", b"dados-2", b"dados-3", b"dados-4"];
        let tree = MerkleTree::from_data_chunks(&chunks);
        let root = tree.root();

        let proof = tree.generate_proof(0).unwrap();
        let fake_hash = blake3_hash(b"dados-adulterados");

        // Deve falhar na validação da prova
        assert!(!MerkleTree::verify_proof(&root, &fake_hash, &proof));
    }
}
