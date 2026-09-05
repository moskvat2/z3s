use super::format::{
    BlockHeader, ExtentHeader, FormatError, BLOCK_HEADER_SIZE, EXTENT_HEADER_SIZE,
    SECTOR_ALIGNMENT,
};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;
use z3s_common::hash::blake3_hash;

#[derive(Debug, Error)]
pub enum ExtentError {
    #[error("Erro de I/O em arquivo de extensão: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erro de formato binário: {0}")]
    Format(#[from] FormatError),

    #[error("Capacidade máxima do Extent excedida")]
    ExtentFull,
}

/// Gerenciador de leitura e gravação de um arquivo de Extensão físico
pub struct ExtentFile {
    pub path: PathBuf,
    pub header: ExtentHeader,
    file: File,
    current_offset: u64,
}

impl ExtentFile {
    /// Cria um novo arquivo de extensão no disco com cabeçalho inicial de 4096 bytes
    pub fn create(path: impl AsRef<Path>, extent_id: Uuid, max_capacity_bytes: u64) -> Result<Self, ExtentError> {
        let path = path.as_ref().to_path_buf();
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;

        let header = ExtentHeader::new(extent_id, max_capacity_bytes);
        let serialized_header = header.serialize();
        file.write_all(&serialized_header)?;
        file.flush()?;

        Ok(Self {
            path,
            header,
            file,
            current_offset: EXTENT_HEADER_SIZE as u64,
        })
    }

    /// Abre um arquivo de extensão existente e valida seu cabeçalho
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ExtentError> {
        let path = path.as_ref().to_path_buf();
        let mut file = OpenOptions::new().read(true).write(true).open(&path)?;

        let mut header_buf = [0u8; EXTENT_HEADER_SIZE];
        file.read_exact(&mut header_buf)?;

        let header = ExtentHeader::deserialize(&header_buf)?;
        let file_len = file.metadata()?.len();

        Ok(Self {
            path,
            header,
            file,
            current_offset: file_len,
        })
    }

    /// Grava um Shard no Extent com alinhamento de 4096 bytes e cabeçalho de 64 bytes
    pub fn append_shard(
        &mut self,
        shard_id: Uuid,
        shard_index: u32,
        total_shards: u32,
        payload: &[u8],
    ) -> Result<BlockHeader, ExtentError> {
        let payload_len = payload.len() as u64;
        let total_block_size = BLOCK_HEADER_SIZE as u64 + payload_len;

        // Calcula padding para alinhar ao setor de 4096 bytes
        let padding_needed = (SECTOR_ALIGNMENT as u64 - (total_block_size % SECTOR_ALIGNMENT as u64)) % SECTOR_ALIGNMENT as u64;
        let total_write_size = total_block_size + padding_needed;

        if self.current_offset + total_write_size > self.header.max_capacity_bytes {
            return Err(ExtentError::ExtentFull);
        }

        let checksum = blake3_hash(payload);
        let block_header = BlockHeader::new(
            shard_id,
            shard_index,
            total_shards,
            self.current_offset,
            payload_len,
            checksum,
        );

        let serialized_header = block_header.serialize();

        self.file.seek(SeekFrom::Start(self.current_offset))?;
        self.file.write_all(&serialized_header)?;
        self.file.write_all(payload)?;

        if padding_needed > 0 {
            let padding = vec![0u8; padding_needed as usize];
            self.file.write_all(&padding)?;
        }

        self.file.flush()?;

        let written_offset = self.current_offset;
        self.current_offset += total_write_size;

        let mut confirmed_header = block_header;
        confirmed_header.offset_in_extent = written_offset;
        Ok(confirmed_header)
    }

    /// Lê um Shard e valida o checksum BLAKE3 on-the-fly (detecção de Bitrot)
    pub fn read_shard(&mut self, offset: u64) -> Result<(BlockHeader, Vec<u8>), ExtentError> {
        self.file.seek(SeekFrom::Start(offset))?;

        let mut header_buf = [0u8; BLOCK_HEADER_SIZE];
        self.file.read_exact(&mut header_buf)?;

        let block_header = BlockHeader::deserialize(&header_buf, offset)?;

        let mut payload = vec![0u8; block_header.payload_length as usize];
        self.file.read_exact(&mut payload)?;

        // Verificação estrita de integridade contra corrupção silenciosa
        let calculated_checksum = blake3_hash(&payload);
        if calculated_checksum != block_header.checksum_blake3 {
            return Err(ExtentError::Format(FormatError::ChecksumMismatch {
                expected: hex::encode(block_header.checksum_blake3),
                calculated: hex::encode(calculated_checksum),
            }));
        }

        Ok((block_header, payload))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extent_lifecycle_create_append_read_verify() {
        let temp_dir = tempfile::tempdir().unwrap();
        let extent_path = temp_dir.path().join("extent-001.z3se");
        let extent_id = Uuid::new_v4();

        // 1. Cria o Extent File com 10MB de capacidade
        let mut extent = ExtentFile::create(&extent_path, extent_id, 10 * 1024 * 1024).unwrap();

        // 2. Grava Shard 1
        let shard_id_1 = Uuid::new_v4();
        let payload_1 = b"Conteudo do primeiro shard gravado com sucesso!";
        let header_1 = extent.append_shard(shard_id_1, 0, 4, payload_1).unwrap();

        // 3. Grava Shard 2
        let shard_id_2 = Uuid::new_v4();
        let payload_2 = b"Conteudo do segundo shard em bloco subsequente!";
        let header_2 = extent.append_shard(shard_id_2, 1, 4, payload_2).unwrap();

        // 4. Lê de volta e valida integridade
        let (read_header_1, read_payload_1) = extent.read_shard(header_1.offset_in_extent).unwrap();
        assert_eq!(read_header_1.shard_id, shard_id_1);
        assert_eq!(read_payload_1, payload_1);

        let (read_header_2, read_payload_2) = extent.read_shard(header_2.offset_in_extent).unwrap();
        assert_eq!(read_header_2.shard_id, shard_id_2);
        assert_eq!(read_payload_2, payload_2);
    }

    #[test]
    fn test_extent_detects_bitrot_corruption() {
        let temp_dir = tempfile::tempdir().unwrap();
        let extent_path = temp_dir.path().join("extent-bitrot.z3se");
        let extent_id = Uuid::new_v4();

        let mut extent = ExtentFile::create(&extent_path, extent_id, 10 * 1024 * 1024).unwrap();
        let shard_id = Uuid::new_v4();
        let payload = b"Payload seguro que sofrera corrupcao no disco.";
        let header = extent.append_shard(shard_id, 0, 4, payload).unwrap();

        // Simula corrupção silenciosa no disco (altera 1 byte do payload gravado)
        let mut corrupt_file = OpenOptions::new().read(true).write(true).open(&extent_path).unwrap();
        corrupt_file.seek(SeekFrom::Start(header.offset_in_extent + BLOCK_HEADER_SIZE as u64 + 5)).unwrap();
        corrupt_file.write_all(b"X").unwrap();
        corrupt_file.flush().unwrap();

        // Tenta ler através do ExtentFile -> Deve falhar com ChecksumMismatch
        let result = extent.read_shard(header.offset_in_extent);
        assert!(matches!(
            result,
            Err(ExtentError::Format(FormatError::ChecksumMismatch { .. }))
        ));
    }
}
