use md5::{Digest, Md5};

/// Calcula hash BLAKE3 (usado para detecção ultra-rápida de integridade interna e bitrot)
pub fn blake3_hash(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

/// Calcula ETag clássico S3 baseado em MD5
pub fn calculate_md5_etag(data: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Calcula checksum CRC32C acelerado por hardware
pub fn calculate_crc32c(data: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(data);
    hasher.finalize()
}
