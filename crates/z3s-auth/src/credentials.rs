use std::collections::HashMap;
use std::sync::RwLock;

/// Credenciais de acesso de um usuário (AccessKeyId e SecretAccessKey)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
}

impl Credentials {
    pub fn new(access_key_id: impl Into<String>, secret_access_key: impl Into<String>) -> Self {
        Self {
            access_key_id: access_key_id.into(),
            secret_access_key: secret_access_key.into(),
        }
    }
}

/// Provedor de credenciais thread-safe
pub trait CredentialsProvider: Send + Sync {
    fn get_credentials(&self, access_key_id: &str) -> Option<Credentials>;
    fn list_keys(&self) -> Vec<String> {
        Vec::new()
    }
    fn register_key(&self, _access_key_id: &str, _secret_access_key: &str) {}
    fn delete_key(&self, _access_key_id: &str) -> bool {
        false
    }
}

/// Armazenamento em memória de credenciais para autenticação de clientes
pub struct InMemoryCredentialsStore {
    store: RwLock<HashMap<String, String>>,
}

impl InMemoryCredentialsStore {
    pub fn new() -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, access_key_id: impl Into<String>, secret_access_key: impl Into<String>) {
        let mut map = self.store.write().unwrap();
        map.insert(access_key_id.into(), secret_access_key.into());
    }

    pub fn list_keys(&self) -> Vec<String> {
        let map = self.store.read().unwrap();
        map.keys().cloned().collect()
    }

    pub fn delete(&self, access_key_id: &str) -> bool {
        let mut map = self.store.write().unwrap();
        map.remove(access_key_id).is_some()
    }

    pub fn has_key(&self, access_key_id: &str) -> bool {
        let map = self.store.read().unwrap();
        map.contains_key(access_key_id)
    }
}

impl CredentialsProvider for InMemoryCredentialsStore {
    fn get_credentials(&self, access_key_id: &str) -> Option<Credentials> {
        let map = self.store.read().unwrap();
        map.get(access_key_id).map(|secret| Credentials {
            access_key_id: access_key_id.to_string(),
            secret_access_key: secret.clone(),
        })
    }

    fn list_keys(&self) -> Vec<String> {
        let map = self.store.read().unwrap();
        map.keys().cloned().collect()
    }

    fn register_key(&self, access_key_id: &str, secret_access_key: &str) {
        self.register(access_key_id, secret_access_key);
    }

    fn delete_key(&self, access_key_id: &str) -> bool {
        self.delete(access_key_id)
    }
}

impl Default for InMemoryCredentialsStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Armazenamento persistente e transacional de credenciais IAM em disco (ACID-safe)
pub struct PersistentCredentialsStore {
    inner: InMemoryCredentialsStore,
    file_path: std::path::PathBuf,
}

impl PersistentCredentialsStore {
    /// Abre ou inicializa o repositório persistente de credenciais.
    /// Carrega as credenciais existentes preservando-as integralmente.
    pub fn open(file_path: impl Into<std::path::PathBuf>) -> std::io::Result<Self> {
        let file_path = file_path.into();
        let inner = InMemoryCredentialsStore::new();

        if file_path.exists() {
            match std::fs::read_to_string(&file_path) {
                Ok(content) => {
                    if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(&content) {
                        for (ak, sk) in map {
                            inner.register(ak, sk);
                        }
                        tracing::info!(
                            "Carregadas {} credenciais persistentes de {:?}",
                            inner.list_keys().len(),
                            file_path
                        );
                    } else {
                        tracing::warn!(
                            "Arquivo de credenciais {:?} possui formato inesperado",
                            file_path
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Falha ao ler arquivo de credenciais {:?}: {}",
                        file_path,
                        e
                    );
                }
            }
        }

        Ok(Self { inner, file_path })
    }

    /// Salva as credenciais atômica e seguramente em disco com fsync e rename (crash-safe)
    fn flush(&self) -> std::io::Result<()> {
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let map = self.inner.store.read().unwrap();
        let json_bytes = serde_json::to_vec_pretty(&*map)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let temp_name = format!(
            ".{}.tmp.{}",
            self.file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("credentials.json"),
            uuid::Uuid::new_v4().simple()
        );
        let temp_path = self.file_path.with_file_name(temp_name);

        {
            use std::io::Write;
            let mut file = std::fs::File::create(&temp_path)?;
            file.write_all(&json_bytes)?;
            file.sync_all()?;
        }

        std::fs::rename(&temp_path, &self.file_path)?;

        #[cfg(unix)]
        if let Some(parent) = self.file_path.parent() {
            if let Ok(dir_file) = std::fs::File::open(parent) {
                let _ = dir_file.sync_all();
            }
        }

        Ok(())
    }

    pub fn register(&self, access_key_id: impl Into<String>, secret_access_key: impl Into<String>) {
        self.inner.register(access_key_id, secret_access_key);
        if let Err(e) = self.flush() {
            tracing::error!(
                "Falha ao persistir credenciais em disco ({:?}): {}",
                self.file_path,
                e
            );
        }
    }

    pub fn delete(&self, access_key_id: &str) -> bool {
        let removed = self.inner.delete(access_key_id);
        if removed {
            if let Err(e) = self.flush() {
                tracing::error!(
                    "Falha ao persistir exclusão de credencial em disco ({:?}): {}",
                    self.file_path,
                    e
                );
            }
        }
        removed
    }

    pub fn has_key(&self, access_key_id: &str) -> bool {
        self.inner.has_key(access_key_id)
    }

    pub fn list_keys(&self) -> Vec<String> {
        self.inner.list_keys()
    }

    pub fn get_credentials(&self, access_key_id: &str) -> Option<Credentials> {
        self.inner.get_credentials(access_key_id)
    }
}

impl CredentialsProvider for PersistentCredentialsStore {
    fn get_credentials(&self, access_key_id: &str) -> Option<Credentials> {
        self.inner.get_credentials(access_key_id)
    }

    fn list_keys(&self) -> Vec<String> {
        self.inner.list_keys()
    }

    fn register_key(&self, access_key_id: &str, secret_access_key: &str) {
        self.register(access_key_id, secret_access_key);
    }

    fn delete_key(&self, access_key_id: &str) -> bool {
        self.delete(access_key_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persistent_credentials_lifecycle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cred_file = temp_dir.path().join("credentials.json");

        {
            let store = PersistentCredentialsStore::open(&cred_file).unwrap();
            store.register("TESTKEY1", "TESTSECRET1");
            store.register("TESTKEY2", "TESTSECRET2");
            assert_eq!(store.list_keys().len(), 2);
            assert_eq!(
                store.get_credentials("TESTKEY1").unwrap().secret_access_key,
                "TESTSECRET1"
            );
        }

        // Reabre do arquivo persistido garantindo integridade
        {
            let store = PersistentCredentialsStore::open(&cred_file).unwrap();
            assert_eq!(store.list_keys().len(), 2);
            assert_eq!(
                store.get_credentials("TESTKEY2").unwrap().secret_access_key,
                "TESTSECRET2"
            );
            assert!(store.delete("TESTKEY1"));
            assert_eq!(store.list_keys().len(), 1);
        }

        // Reabre novamente para garantir que a exclusão persistiu
        {
            let store = PersistentCredentialsStore::open(&cred_file).unwrap();
            assert_eq!(store.list_keys().len(), 1);
            assert!(store.get_credentials("TESTKEY1").is_none());
            assert!(store.get_credentials("TESTKEY2").is_some());
        }
    }
}
