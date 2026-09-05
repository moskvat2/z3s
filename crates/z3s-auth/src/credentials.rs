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
}

impl CredentialsProvider for InMemoryCredentialsStore {
    fn get_credentials(&self, access_key_id: &str) -> Option<Credentials> {
        let map = self.store.read().unwrap();
        map.get(access_key_id).map(|secret| Credentials {
            access_key_id: access_key_id.to_string(),
            secret_access_key: secret.clone(),
        })
    }
}

impl Default for InMemoryCredentialsStore {
    fn default() -> Self {
        Self::new()
    }
}
