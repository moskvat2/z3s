use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::rand_core::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KmsError {
    #[error("Chave mestra (KEK) não encontrada: {0}")]
    KeyNotFound(String),
    #[error("Falha de criptografia: {0}")]
    EncryptionFailed(String),
    #[error("Falha de decriptografia ou dados corrompidos: {0}")]
    DecryptionFailed(String),
    #[error("Tamanho de chave inválido: esperado 32 bytes (AES-256)")]
    InvalidKeyLength,
    #[error("Formato de IV inválido: esperado 12 bytes")]
    InvalidIvLength,
    #[error("Erro de codificação hex: {0}")]
    HexError(#[from] hex::FromHexError),
}

/// Metadados de uma chave de dados (DEK) criptografada via Envelope Encryption
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedDataKey {
    pub key_id: String,
    pub encrypted_dek_hex: String,
    pub iv_hex: String,
}

/// Mecanismo de Gerenciamento de Chaves (KMS) com Envelope Encryption
pub struct KmsEngine {
    master_keys: RwLock<HashMap<String, [u8; 32]>>,
}

impl KmsEngine {
    /// Inicializa o motor KMS com uma chave mestra padrão ("aws/s3" e "default")
    pub fn new() -> Self {
        let engine = Self {
            master_keys: RwLock::new(HashMap::new()),
        };
        // Gera chave mestra padrão de 256 bits
        let mut default_key = [0u8; 32];
        OsRng.fill_bytes(&mut default_key);
        engine.register_master_key("default", default_key);
        engine.register_master_key("aws/s3", default_key);
        engine
    }

    /// Registra ou carrega uma chave mestra (KEK) pelo ID
    pub fn register_master_key(&self, key_id: &str, key_bytes: [u8; 32]) {
        self.master_keys
            .write()
            .unwrap()
            .insert(key_id.to_string(), key_bytes);
    }

    /// Gera uma chave de dados efêmera (DEK) e a encapsula com a chave mestra (Envelope Encryption)
    pub fn generate_data_key(&self, master_key_id: &str) -> Result<([u8; 32], EncryptedDataKey), KmsError> {
        let master_key_bytes = {
            let keys = self.master_keys.read().unwrap();
            *keys
                .get(master_key_id)
                .or_else(|| keys.get("default"))
                .ok_or_else(|| KmsError::KeyNotFound(master_key_id.to_string()))?
        };

        // 1. Gera DEK aleatória de 32 bytes (256 bits)
        let mut raw_dek = [0u8; 32];
        OsRng.fill_bytes(&mut raw_dek);

        // 2. Gera IV aleatório de 12 bytes para criptografar a DEK
        let mut iv = [0u8; 12];
        OsRng.fill_bytes(&mut iv);

        // 3. Criptografa a DEK com a Master Key usando AES-256-GCM
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&master_key_bytes));
        let nonce = Nonce::from_slice(&iv);
        let ciphertext = cipher
            .encrypt(nonce, raw_dek.as_ref())
            .map_err(|e| KmsError::EncryptionFailed(e.to_string()))?;

        let encrypted_key = EncryptedDataKey {
            key_id: master_key_id.to_string(),
            encrypted_dek_hex: hex::encode(ciphertext),
            iv_hex: hex::encode(iv),
        };

        Ok((raw_dek, encrypted_key))
    }

    /// Decriptografa uma DEK usando a chave mestra (KEK) correspondente
    pub fn decrypt_data_key(&self, encrypted: &EncryptedDataKey) -> Result<[u8; 32], KmsError> {
        let master_key_bytes = {
            let keys = self.master_keys.read().unwrap();
            *keys
                .get(&encrypted.key_id)
                .or_else(|| keys.get("default"))
                .ok_or_else(|| KmsError::KeyNotFound(encrypted.key_id.clone()))?
        };

        let ciphertext = hex::decode(&encrypted.encrypted_dek_hex)?;
        let iv = hex::decode(&encrypted.iv_hex)?;
        if iv.len() != 12 {
            return Err(KmsError::InvalidIvLength);
        }

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&master_key_bytes));
        let nonce = Nonce::from_slice(&iv);
        let decrypted_bytes = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|e| KmsError::DecryptionFailed(e.to_string()))?;

        if decrypted_bytes.len() != 32 {
            return Err(KmsError::InvalidKeyLength);
        }

        let mut dek = [0u8; 32];
        dek.copy_from_slice(&decrypted_bytes);
        Ok(dek)
    }

    /// Criptografa o payload do objeto com a DEK usando AES-256-GCM autenticado
    pub fn encrypt_payload(dek: &[u8; 32], plaintext: &[u8]) -> Result<(Vec<u8>, [u8; 12]), KmsError> {
        let mut iv = [0u8; 12];
        OsRng.fill_bytes(&mut iv);

        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(dek));
        let nonce = Nonce::from_slice(&iv);
        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| KmsError::EncryptionFailed(e.to_string()))?;

        Ok((ciphertext, iv))
    }

    /// Decriptografa e valida a autenticidade do payload do objeto com a DEK usando AES-256-GCM
    pub fn decrypt_payload(dek: &[u8; 32], ciphertext: &[u8], iv: &[u8; 12]) -> Result<Vec<u8>, KmsError> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(dek));
        let nonce = Nonce::from_slice(iv);
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| KmsError::DecryptionFailed(e.to_string()))?;

        Ok(plaintext)
    }
}

impl Default for KmsEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_encryption_full_cycle() {
        let kms = KmsEngine::new();
        let payload = b"Documento confidencial ultra secreto com Z3S Storage";

        // 1. Gera DEK e encapsula com KEK
        let (raw_dek, encrypted_dek) = kms.generate_data_key("aws/s3").unwrap();

        // 2. Criptografa o payload com a DEK
        let (ciphertext, iv) = KmsEngine::encrypt_payload(&raw_dek, payload).unwrap();
        assert_ne!(ciphertext, payload);

        // 3. Simula perda da DEK em memória e recupera desencapsulando com o KMS
        let recovered_dek = kms.decrypt_data_key(&encrypted_dek).unwrap();
        assert_eq!(recovered_dek, raw_dek);

        // 4. Decriptografa o payload original
        let decrypted = KmsEngine::decrypt_payload(&recovered_dek, &ciphertext, &iv).unwrap();
        assert_eq!(decrypted, payload);
    }

    #[test]
    fn test_corrupted_ciphertext_fails_decryption() {
        let kms = KmsEngine::new();
        let payload = b"Dados com verificacao de integridade AES-GCM";

        let (raw_dek, _) = kms.generate_data_key("default").unwrap();
        let (mut ciphertext, iv) = KmsEngine::encrypt_payload(&raw_dek, payload).unwrap();

        // Corrompe 1 byte do ciphertext (simula bitrot ou adulteração maliciosa)
        ciphertext[0] ^= 0xFF;

        let result = KmsEngine::decrypt_payload(&raw_dek, &ciphertext, &iv);
        assert!(result.is_err());
    }
}
