use thiserror::Error;
use uuid::Uuid;

pub const EXTENT_MAGIC: [u8; 4] = *b"Z3SE"; // Z3S Extent File
pub const BLOCK_MAGIC: [u8; 4] = *b"Z3SB";  // Z3S Storage Block
pub const EXTENT_HEADER_SIZE: usize = 4096;  // 4KB alinhado
pub const BLOCK_HEADER_SIZE: usize = 64;     // 64B alinhado
pub const SECTOR_ALIGNMENT: usize = 4096;     // 4KB Direct I/O

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FormatError {
    #[error("Magic bytes do cabeçalho de extensão inválidos: {0:?}")]
    InvalidExtentMagic([u8; 4]),

    #[error("Magic bytes do cabeçalho de bloco inválidos: {0:?}")]
    InvalidBlockMagic([u8; 4]),

    #[error("Versão de formato de extensão não suportada: {0}")]
    UnsupportedVersion(u16),

    #[error("Buffer insuficiente para serialização/deserialização: esperado {expected}, recebido {actual}")]
    BufferTooSmall { expected: usize, actual: usize },

    #[error("Corrupção detectada! Checksum BLAKE3 não coincide: esperado {expected}, calculado {calculated}")]
    ChecksumMismatch { expected: String, calculated: String },
}

/// Cabeçalho imutável do arquivo de Extensão física (4096 bytes)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtentHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub extent_id: Uuid,
    pub block_alignment: u32,
    pub max_capacity_bytes: u64,
    pub created_at: i64,
}

impl ExtentHeader {
    pub fn new(extent_id: Uuid, max_capacity_bytes: u64) -> Self {
        Self {
            magic: EXTENT_MAGIC,
            version: 1,
            extent_id,
            block_alignment: SECTOR_ALIGNMENT as u32,
            max_capacity_bytes,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    pub fn serialize(&self) -> [u8; EXTENT_HEADER_SIZE] {
        let mut buffer = [0u8; EXTENT_HEADER_SIZE];
        buffer[0..4].copy_from_slice(&self.magic);
        buffer[4..6].copy_from_slice(&self.version.to_le_bytes());
        buffer[6..22].copy_from_slice(self.extent_id.as_bytes());
        buffer[22..26].copy_from_slice(&self.block_alignment.to_le_bytes());
        buffer[26..34].copy_from_slice(&self.max_capacity_bytes.to_le_bytes());
        buffer[34..42].copy_from_slice(&self.created_at.to_le_bytes());
        buffer
    }

    pub fn deserialize(buffer: &[u8]) -> Result<Self, FormatError> {
        if buffer.len() < EXTENT_HEADER_SIZE {
            return Err(FormatError::BufferTooSmall {
                expected: EXTENT_HEADER_SIZE,
                actual: buffer.len(),
            });
        }

        let magic: [u8; 4] = buffer[0..4].try_into().unwrap();
        if magic != EXTENT_MAGIC {
            return Err(FormatError::InvalidExtentMagic(magic));
        }

        let version = u16::from_le_bytes(buffer[4..6].try_into().unwrap());
        if version != 1 {
            return Err(FormatError::UnsupportedVersion(version));
        }

        let extent_id = Uuid::from_bytes(buffer[6..22].try_into().unwrap());
        let block_alignment = u32::from_le_bytes(buffer[22..26].try_into().unwrap());
        let max_capacity_bytes = u64::from_le_bytes(buffer[26..34].try_into().unwrap());
        let created_at = i64::from_le_bytes(buffer[34..42].try_into().unwrap());

        Ok(Self {
            magic,
            version,
            extent_id,
            block_alignment,
            max_capacity_bytes,
            created_at,
        })
    }
}

/// Cabeçalho binário de cada Shard/Bloco gravado dentro do Extent (64 bytes)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockHeader {
    pub magic: [u8; 4],
    pub shard_id: Uuid,
    pub shard_index: u32,
    pub total_shards: u32,
    pub offset_in_extent: u64,
    pub payload_length: u64,
    pub checksum_blake3: [u8; 32],
}

impl BlockHeader {
    pub fn new(
        shard_id: Uuid,
        shard_index: u32,
        total_shards: u32,
        offset_in_extent: u64,
        payload_length: u64,
        checksum_blake3: [u8; 32],
    ) -> Self {
        Self {
            magic: BLOCK_MAGIC,
            shard_id,
            shard_index,
            total_shards,
            offset_in_extent,
            payload_length,
            checksum_blake3,
        }
    }

    pub fn serialize(&self) -> [u8; BLOCK_HEADER_SIZE] {
        let mut buffer = [0u8; BLOCK_HEADER_SIZE];
        buffer[0..4].copy_from_slice(&self.magic);
        buffer[4..20].copy_from_slice(self.shard_id.as_bytes());
        buffer[20..24].copy_from_slice(&self.shard_index.to_le_bytes());
        buffer[24..28].copy_from_slice(&self.total_shards.to_le_bytes());
        buffer[28..36].copy_from_slice(&self.offset_in_extent.to_le_bytes());
        buffer[36..44].copy_from_slice(&self.payload_length.to_le_bytes());
        buffer[44..76.min(BLOCK_HEADER_SIZE)].copy_from_slice(&self.checksum_blake3[0..20]);
        // Para caber os 32 bytes do BLAKE3 perfeitamente:
        // 0..4: magic (4)
        // 4..20: shard_id (16)
        // 20..24: shard_index (4)
        // 24..28: total_shards (4)
        // 28..32: flags/reserved (4)
        // 32..64: checksum_blake3 (32)
        // Total = 64 bytes
        let mut buf = [0u8; BLOCK_HEADER_SIZE];
        buf[0..4].copy_from_slice(&self.magic);
        buf[4..20].copy_from_slice(self.shard_id.as_bytes());
        buf[20..24].copy_from_slice(&self.shard_index.to_le_bytes());
        buf[24..28].copy_from_slice(&self.total_shards.to_le_bytes());
        buf[28..32].copy_from_slice(&(self.payload_length as u32).to_le_bytes());
        buf[32..64].copy_from_slice(&self.checksum_blake3);
        buf
    }

    pub fn deserialize(buffer: &[u8], offset_in_extent: u64) -> Result<Self, FormatError> {
        if buffer.len() < BLOCK_HEADER_SIZE {
            return Err(FormatError::BufferTooSmall {
                expected: BLOCK_HEADER_SIZE,
                actual: buffer.len(),
            });
        }

        let magic: [u8; 4] = buffer[0..4].try_into().unwrap();
        if magic != BLOCK_MAGIC {
            return Err(FormatError::InvalidBlockMagic(magic));
        }

        let shard_id = Uuid::from_bytes(buffer[4..20].try_into().unwrap());
        let shard_index = u32::from_le_bytes(buffer[20..24].try_into().unwrap());
        let total_shards = u32::from_le_bytes(buffer[24..28].try_into().unwrap());
        let payload_length = u32::from_le_bytes(buffer[28..32].try_into().unwrap()) as u64;
        let checksum_blake3: [u8; 32] = buffer[32..64].try_into().unwrap();

        Ok(Self {
            magic,
            shard_id,
            shard_index,
            total_shards,
            offset_in_extent,
            payload_length,
            checksum_blake3,
        })
    }
}
