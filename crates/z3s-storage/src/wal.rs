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

    /// Grava um lote de operações no WAL com checksum CRC32 por entrada e um único sync_data em disco
    pub fn append_batch(&mut self, records: &[WalRecord]) -> Result<(), WalError> {
        if records.is_empty() {
            return Ok(());
        }

        for record in records {
            let serialized = serde_json::to_vec(record)?;
            let length = serialized.len() as u32;
            let crc = calculate_crc32c(&serialized);

            // Formato binário do log: [Len 4B][CRC32 4B][Payload JSON Bytes]
            self.writer.write_all(&length.to_le_bytes())?;
            self.writer.write_all(&crc.to_le_bytes())?;
            self.writer.write_all(&serialized)?;
        }

        self.writer.flush()?;
        self.writer.get_ref().sync_data()?;

        Ok(())
    }

    /// Grava uma operação no WAL com checksum CRC32 e sincronização física em disco
    pub fn append(&mut self, record: &WalRecord) -> Result<(), WalError> {
        self.append_batch(std::slice::from_ref(record))
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

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex};

/// Coordenador de Group Commit para agregação e coalescência de escritas concorrentes no WAL
pub struct GroupCommitWal {
    wal: Mutex<WriteAheadLog>,
    state: Mutex<GroupCommitState>,
    cv: Condvar,
    last_synced_seq: AtomicU64,
}

struct GroupCommitState {
    current_seq: u64,
    pending: Vec<WalRecord>,
    committer_active: bool,
    last_error: Option<String>,
}

impl GroupCommitWal {
    pub fn new(wal: WriteAheadLog) -> Self {
        Self {
            wal: Mutex::new(wal),
            state: Mutex::new(GroupCommitState {
                current_seq: 0,
                pending: Vec::new(),
                committer_active: false,
                last_error: None,
            }),
            cv: Condvar::new(),
            last_synced_seq: AtomicU64::new(0),
        }
    }

    /// Submete registros para sincronização durável com Group Commit concorrente
    pub fn append_group(&self, records: &[WalRecord]) -> Result<(), WalError> {
        if records.is_empty() {
            return Ok(());
        }

        let my_seq = {
            let mut state = self.state.lock().unwrap();
            if let Some(ref err) = state.last_error {
                return Err(WalError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    err.clone(),
                )));
            }
            state.current_seq += 1;
            let seq = state.current_seq;
            state.pending.extend_from_slice(records);
            seq
        };

        loop {
            let mut state = self.state.lock().unwrap();
            if let Some(ref err) = state.last_error {
                return Err(WalError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    err.clone(),
                )));
            }

            if self.last_synced_seq.load(Ordering::Acquire) >= my_seq {
                return Ok(());
            }

            if !state.committer_active {
                state.committer_active = true;
                let max_seq = state.current_seq;
                let to_commit = std::mem::take(&mut state.pending);
                drop(state);

                let sync_result = {
                    let mut wal = self.wal.lock().unwrap();
                    wal.append_batch(&to_commit)
                };

                let mut state = self.state.lock().unwrap();
                state.committer_active = false;
                match sync_result {
                    Ok(()) => {
                        self.last_synced_seq.store(max_seq, Ordering::Release);
                        self.cv.notify_all();
                        return Ok(());
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        state.last_error = Some(err_str);
                        self.cv.notify_all();
                        return Err(e);
                    }
                }
            } else {
                state = self.cv.wait(state).unwrap();
            }
        }
    }

    pub fn truncate(&self) -> Result<(), WalError> {
        let mut state = self.state.lock().unwrap();
        state.pending.clear();
        state.last_error = None;
        let mut wal = self.wal.lock().unwrap();
        wal.truncate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

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

    #[test]
    fn test_group_commit_concurrent_replay() {
        let temp_dir = tempfile::tempdir().unwrap();
        let wal_path = temp_dir.path().join("storage.wal");

        let raw_wal = WriteAheadLog::open(&wal_path).unwrap();
        let group_wal = Arc::new(GroupCommitWal::new(raw_wal));

        let num_threads = 10;
        let records_per_thread = 20;
        let mut handles = Vec::new();

        for t in 0..num_threads {
            let gw = group_wal.clone();
            handles.push(thread::spawn(move || {
                for i in 0..records_per_thread {
                    let record = WalRecord::ShardCommitted {
                        shard_id: Uuid::new_v4(),
                        extent_id: Uuid::new_v4(),
                        offset_in_extent: (t * 1000 + i) as u64,
                        payload_length: 512,
                        checksum_blake3: [7u8; 32],
                    };
                    gw.append_group(&[record]).unwrap();
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let replayed = WriteAheadLog::replay(&wal_path).unwrap();
        assert_eq!(replayed.len(), num_threads * records_per_thread);
    }
}
