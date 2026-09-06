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
