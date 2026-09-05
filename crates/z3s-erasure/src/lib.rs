use reed_solomon_erasure::galois_8::ReedSolomon;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErasureError {
    #[error("Configuração inválida de shards: data_shards ({data}) e parity_shards ({parity}) devem ser > 0")]
    InvalidShardConfig { data: usize, parity: usize },

    #[error("Quantidade insuficiente de shards para reconstrução: recebidos {present}, necessários ao menos {needed}")]
    TooFewShards { present: usize, needed: usize },

    #[error("Erro no motor Reed-Solomon: {0}")]
    EngineError(String),

    #[error("Comprimento de shard inconsistente ou inválido")]
    CorruptedShardLength,
}

/// Representa o conjunto de shards codificados de um objeto
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedShards {
    /// Shards originais de dados (K partes)
    pub data_shards: Vec<Vec<u8>>,
    /// Shards de paridade calculados via Reed-Solomon (M partes)
    pub parity_shards: Vec<Vec<u8>>,
    /// Tamanho original do payload em bytes antes do padding
    pub original_len: usize,
    /// Tamanho individual de cada shard (inclui padding para divisão uniforme)
    pub shard_len: usize,
}

impl EncodedShards {
    /// Retorna todos os shards concatenados na ordem [Dados (0..K), Paridade (0..M)]
    pub fn all_shards(self) -> Vec<Vec<u8>> {
        let mut all = self.data_shards;
        all.extend(self.parity_shards);
        all
    }
}

/// Motor de Codificação de Apagamento (Reed-Solomon $K+M$)
#[derive(Clone)]
pub struct ErasureEngine {
    data_shards: usize,
    parity_shards: usize,
    rs: ReedSolomon,
}

impl ErasureEngine {
    /// Inicializa o motor com $K$ dados e $M$ paridades
    pub fn new(data_shards: usize, parity_shards: usize) -> Result<Self, ErasureError> {
        if data_shards == 0 || parity_shards == 0 {
            return Err(ErasureError::InvalidShardConfig {
                data: data_shards,
                parity: parity_shards,
            });
        }

        let rs = ReedSolomon::new(data_shards, parity_shards)
            .map_err(|e| ErasureError::EngineError(e.to_string()))?;

        Ok(Self {
            data_shards,
            parity_shards,
            rs,
        })
    }

    pub fn total_shards(&self) -> usize {
        self.data_shards + self.parity_shards
    }

    pub fn data_shards(&self) -> usize {
        self.data_shards
    }

    pub fn parity_shards(&self) -> usize {
        self.parity_shards
    }

    /// Codifica um buffer de dados em $K$ shards de dados e $M$ shards de paridade
    pub fn encode(&self, data: &[u8]) -> Result<EncodedShards, ErasureError> {
        let original_len = data.len();

        // Calcula o tamanho de cada shard com padding para que todos tenham o mesmo tamanho
        let shard_len = if original_len == 0 {
            1 // shard mínimo de 1 byte para payload vazio
        } else {
            (original_len + self.data_shards - 1) / self.data_shards
        };

        let mut shards: Vec<Vec<u8>> = Vec::with_capacity(self.total_shards());

        // Preenche os K shards de dados
        for i in 0..self.data_shards {
            let start = i * shard_len;
            let mut shard = vec![0u8; shard_len];

            if start < original_len {
                let end = (start + shard_len).min(original_len);
                shard[..(end - start)].copy_from_slice(&data[start..end]);
            }

            shards.push(shard);
        }

        // Aloca os M shards de paridade
        for _ in 0..self.parity_shards {
            shards.push(vec![0u8; shard_len]);
        }

        // Executa a computação de paridade Reed-Solomon acelerada por SIMD
        self.rs
            .encode(&mut shards)
            .map_err(|e| ErasureError::EngineError(e.to_string()))?;

        let parity_shards = shards.split_off(self.data_shards);
        let data_shards = shards;

        Ok(EncodedShards {
            data_shards,
            parity_shards,
            original_len,
            shard_len,
        })
    }

    /// Reconstrói o payload original a partir de uma lista de shards onde shards perdidos são `None`.
    /// Requer pelo menos $K$ shards presentes (dados ou paridade).
    pub fn reconstruct(
        &self,
        shards: &mut [Option<Vec<u8>>],
        original_len: usize,
    ) -> Result<Vec<u8>, ErasureError> {
        if shards.len() != self.total_shards() {
            return Err(ErasureError::EngineError(format!(
                "Tamanho do array de shards inválido: esperado {}, recebido {}",
                self.total_shards(),
                shards.len()
            )));
        }

        let present_count = shards.iter().filter(|s| s.is_some()).count();
        if present_count < self.data_shards {
            return Err(ErasureError::TooFewShards {
                present: present_count,
                needed: self.data_shards,
            });
        }

        // Executa a reconstrução via Reed-Solomon
        self.rs
            .reconstruct(shards)
            .map_err(|e| ErasureError::EngineError(e.to_string()))?;

        // Concatena os K shards de dados reconstruídos até atingir original_len
        let mut reconstructed_data = Vec::with_capacity(original_len);
        for shard_opt in shards.iter().take(self.data_shards) {
            let shard = shard_opt
                .as_ref()
                .ok_or(ErasureError::CorruptedShardLength)?;
            reconstructed_data.extend_from_slice(shard);
        }

        reconstructed_data.truncate(original_len);
        Ok(reconstructed_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_and_reconstruct_no_loss() {
        let engine = ErasureEngine::new(4, 2).unwrap();
        let payload = b"Hello, S3 Distributed Storage System in Rust with Reed-Solomon!";

        let encoded = engine.encode(payload).unwrap();
        assert_eq!(encoded.data_shards.len(), 4);
        assert_eq!(encoded.parity_shards.len(), 2);

        let mut shards: Vec<Option<Vec<u8>>> = encoded
            .all_shards()
            .into_iter()
            .map(Some)
            .collect();

        let reconstructed = engine.reconstruct(&mut shards, payload.len()).unwrap();
        assert_eq!(&reconstructed, payload);
    }

    #[test]
    fn test_reconstruct_with_max_tolerable_loss() {
        // Esquema 4+2: tolera perda de até 2 shards quaisquer
        let engine = ErasureEngine::new(4, 2).unwrap();
        let payload = b"Este payload sera reconstruido com sucesso mesmo perdendo 2 shards inteiros.";

        let encoded = engine.encode(payload).unwrap();
        let mut shards: Vec<Option<Vec<u8>>> = encoded
            .all_shards()
            .into_iter()
            .map(Some)
            .collect();

        // Simula corrupção/perda de 2 shards (1 de dados e 1 de paridade)
        shards[0] = None; // Shard de dado 0 perdido
        shards[4] = None; // Shard de paridade 0 perdido

        let reconstructed = engine.reconstruct(&mut shards, payload.len()).unwrap();
        assert_eq!(&reconstructed, payload);
    }

    #[test]
    fn test_reconstruct_with_all_data_shards_lost_but_parity_surviving() {
        // Esquema 2+2: se perdermos os 2 shards de dados, os 2 de paridade conseguem reconstruir tudo
        let engine = ErasureEngine::new(2, 2).unwrap();
        let payload = b"Recuperacao total apenas atraves dos shards de paridade!";

        let encoded = engine.encode(payload).unwrap();
        let mut shards: Vec<Option<Vec<u8>>> = encoded
            .all_shards()
            .into_iter()
            .map(Some)
            .collect();

        shards[0] = None;
        shards[1] = None;

        let reconstructed = engine.reconstruct(&mut shards, payload.len()).unwrap();
        assert_eq!(&reconstructed, payload);
    }

    #[test]
    fn test_fails_when_loss_exceeds_parity() {
        let engine = ErasureEngine::new(4, 2).unwrap();
        let payload = b"Tentativa que deve falhar porque 3 shards foram destruidos em um esquema 4+2.";

        let encoded = engine.encode(payload).unwrap();
        let mut shards: Vec<Option<Vec<u8>>> = encoded
            .all_shards()
            .into_iter()
            .map(Some)
            .collect();

        // 3 shards perdidos (excede o limite de 2 de paridade)
        shards[0] = None;
        shards[1] = None;
        shards[2] = None;

        let result = engine.reconstruct(&mut shards, payload.len());
        assert!(matches!(result, Err(ErasureError::TooFewShards { present: 3, needed: 4 })));
    }

    #[test]
    fn test_edge_case_empty_and_single_byte() {
        let engine = ErasureEngine::new(3, 2).unwrap();

        // Teste payload vazio
        let empty_payload = b"";
        let encoded = engine.encode(empty_payload).unwrap();
        let mut shards: Vec<Option<Vec<u8>>> = encoded.all_shards().into_iter().map(Some).collect();
        shards[0] = None;
        let reconstructed = engine.reconstruct(&mut shards, empty_payload.len()).unwrap();
        assert_eq!(reconstructed, empty_payload.to_vec());

        // Teste 1 byte
        let one_byte = b"X";
        let encoded = engine.encode(one_byte).unwrap();
        let mut shards: Vec<Option<Vec<u8>>> = encoded.all_shards().into_iter().map(Some).collect();
        shards[1] = None;
        let reconstructed = engine.reconstruct(&mut shards, one_byte.len()).unwrap();
        assert_eq!(&reconstructed, one_byte);
    }
}
