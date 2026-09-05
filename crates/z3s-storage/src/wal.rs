use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;
use z3s_common::hash::calculate_crc32c;

#[derive(Debug, Error)]
pub enum WalError {
    #[error("Erro de I/O no WAL: {0}")]
    Io(#[from] std::io::Error),

    #[error("Erro de serialização JSON no WAL: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Entrada corrompida no WAL: CRC32 mismatch")]
    CorruptedEntry,
}

/// Tipos de operações registradas no Write-Ahead Log
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WalRecord {
    /// Registro de Shard gravado com sucesso no disco
    ShardCommitted {
        shard_id: Uuid,
        extent_id: Uuid,
        offset_in_extent: u64,
        payload_length: u64,
        checksum_blake3: [u8; 32],
    },
    /// Registro de Shard excluído
    ShardDeleted {
        shard_id: Uuid,
    },
}

/// Write-Ahead Log (WAL) para garantia de consistência após falhas de energia (Crash-Consistency)
pub struct WriteAheadLog {
    pub path: PathBuf,
    writer: BufWriter<File>,
}

impl WriteAheadLog {
    /// Inicializa ou abre o WAL no caminho especificado
    pub fn open(path: impl AsRef<Path>) -> Result<Self, WalError> {
        let path = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .open(&path)?;

        Ok(Self {
            path,
            writer: BufWriter::new(file),
        })
    }

    /// Grava uma operação no WAL com checksum CRC32 e sincronização física em disco
    pub fn append(&mut self, record: &WalRecord) -> Result<(), WalError> {
        let serialized = serde_json::to_vec(record)?;
        let length = serialized.len() as u32;
        let crc = calculate_crc32c(&serialized);

        // Formato binário do log: [Len 4B][CRC32 4B][Payload JSON Bytes]
        self.writer.write_all(&length.to_le_bytes())?;
        self.writer.write_all(&crc.to_le_bytes())?;
        self.writer.write_all(&serialized)?;
        self.writer.flush()?;
        self.writer.get_ref().sync_data()?;

        Ok(())
    }

    /// Replay do WAL: lê todas as entradas válidas e reconstrói o estado
    pub fn replay(path: impl AsRef<Path>) -> Result<Vec<WalRecord>, WalError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut records = Vec::new();

        loop {
            let mut len_buf = [0u8; 4];
            match reader.read_exact(&mut len_buf) {
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(WalError::Io(e)),
            }

            let length = u32::from_le_bytes(len_buf) as usize;

            let mut crc_buf = [0u8; 4];
            reader.read_exact(&mut crc_buf)?;
            let expected_crc = u32::from_le_bytes(crc_buf);

            let mut payload = vec![0u8; length];
            reader.read_exact(&mut payload)?;

            let actual_crc = calculate_crc32c(&payload);
            if actual_crc != expected_crc {
                return Err(WalError::CorruptedEntry);
            }

            let record: WalRecord = serde_json::from_slice(&payload)?;
            records.push(record);
        }

        Ok(records)
    }

    /// Trunca/limpa o arquivo WAL após compactação de estado
    pub fn truncate(&mut self) -> Result<(), WalError> {
        self.writer.flush()?;
        let file = self.writer.get_mut();
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        file.sync_all()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wal_append_and_replay() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wal_path = temp_dir.path().join("storage.wal");

        let mut wal = WriteAheadLog::open(&wal_path).unwrap();

        let shard_1 = Uuid::new_v4();
        let extent_1 = Uuid::new_v4();
        let record_1 = WalRecord::ShardCommitted {
            shard_id: shard_1,
            extent_id: extent_1,
            offset_in_extent: 4096,
            payload_length: 1024,
            checksum_blake3: [1u8; 32],
        };

        let shard_2 = Uuid::new_v4();
        let record_2 = WalRecord::ShardDeleted { shard_id: shard_2 };

        wal.append(&record_1).unwrap();
        wal.append(&record_2).unwrap();

        // Faz replay a partir do arquivo
        let replayed = WriteAheadLog::replay(&wal_path).unwrap();
        assert_eq!(replayed.len(), 2);
        assert_eq!(replayed[0], record_1);
        assert_eq!(replayed[1], record_2);
    }
}
