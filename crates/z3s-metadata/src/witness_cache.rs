use std::collections::HashMap;
use std::sync::RwLock;
use z3s_common::manifest::ObjectManifest;

/// Entrada em cache com carimbo de sequência de commit atômico
#[derive(Debug, Clone)]
pub struct WitnessEntry {
    pub manifest: Option<ObjectManifest>,
    pub commit_index: u64,
}

/// Cache de Metadados com Validação Witness (Read-After-Write Consistency)
///
/// Garante que leituras locais (HEAD/GET) nunca retornem dados obsoletos em relação
/// ao commit_index mais recente confirmado pelo cluster Raft.
pub struct WitnessCache {
    entries: RwLock<HashMap<String, WitnessEntry>>,
}

impl Default for WitnessCache {
    fn default() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }
}

impl WitnessCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Atualiza ou insere o manifesto testemunhado com seu commit_index
    pub fn put(&self, bucket: &str, key: &str, manifest: Option<ObjectManifest>, commit_index: u64) {
        let manifest_key = format!("{}/{}", bucket, key);
        let mut map = self.entries.write().unwrap();
        map.insert(
            manifest_key,
            WitnessEntry {
                manifest,
                commit_index,
            },
        );
    }

    /// Obtém o manifesto do cache validando se a versão é ao menos tão recente quanto `min_commit_index`
    pub fn get(&self, bucket: &str, key: &str, min_commit_index: u64) -> Option<Option<ObjectManifest>> {
        let manifest_key = format!("{}/{}", bucket, key);
        let map = self.entries.read().unwrap();
        if let Some(entry) = map.get(&manifest_key) {
            if entry.commit_index >= min_commit_index {
                return Some(entry.manifest.clone());
            }
        }
        None
    }

    /// Invalida uma chave do cache
    pub fn invalidate(&self, bucket: &str, key: &str) {
        let manifest_key = format!("{}/{}", bucket, key);
        let mut map = self.entries.write().unwrap();
        map.remove(&manifest_key);
    }

    /// Limpa todo o cache
    pub fn clear(&self) {
        let mut map = self.entries.write().unwrap();
        map.clear();
    }
}
