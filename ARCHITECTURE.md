# Z3S — Project Architecture & Codebase Structure

Este documento detalha a estrutura de código, a organização modular (multi-crate workspace em Rust) e os fluxos de dados de execução ponta a ponta do projeto **Z3S**.

---

## 1. Estrutura de Diretórios do Projeto

```text
z3s/
├── Cargo.toml                      # Workspace raiz (unifica dependências e perfis de compilação)
├── README.md                       # Apresentação do projeto
├── ROADMAP.md                      # Blueprint arquitetural e fases de execução
├── ARCHITECTURE.md                 # Estrutura modular, crates e fluxos de dados (este arquivo)
│
├── bin/                            # Binários executáveis
│   ├── z3s-server/                 # Executável unificado (All-in-One para dev/small deploy)
│   │   └── src/main.rs
│   ├── z3s-gateway/                # Binário standalone do S3 Gateway (Edge/Proxy stateless)
│   │   └── src/main.rs
│   └── z3s-storaged/               # Binário standalone do Storage Daemon (para nós de disco)
│       └── src/main.rs
│
├── crates/                         # Módulos internos (Crates)
│   │
│   ├── z3s-common/                 # Tipos fundamentais e utilitários
│   │   ├── src/
│   │   │   ├── types.rs            # BucketName, ObjectKey, VersionId, ShardId
│   │   │   ├── error.rs            # Erros padronizados (S3ErrorCode mapping)
│   │   │   ├── hash.rs             # BLAKE3, HighwayHash, MD5 (ETags), CRC32C
│   │   │   └── config.rs           # Configurações globais e parsing TOML/ENV
│   │   └── Cargo.toml
│   │
│   ├── z3s-erasure/                # Motor de Codificação de Apagamento (Reed-Solomon)
│   │   ├── src/
│   │   │   ├── encoder.rs          # Fatiamento em K dados + M paridades
│   │   │   ├── decoder.rs          # Reconstrução de dados a partir de K shards sobreviventes
│   │   │   └── simd.rs             # Aceleração vetorial (AVX-512 / AVX2 / NEON)
│   │   └── Cargo.toml
│   │
│   ├── z3s-storage/                # Engine de Armazenamento Local (ShardStore / Extent Store)
│   │   ├── src/
│   │   │   ├── driver/
│   │   │   │   ├── io_uring.rs     # Driver assíncrono Linux io_uring (Direct I/O)
│   │   │   │   └── posix.rs        # Driver fallback (para sistemas sem io_uring)
│   │   │   ├── extent.rs           # Gerenciador de arquivos de extensão (64MB/128MB)
│   │   │   ├── wal.rs              # Write-Ahead Log para crash-consistency
│   │   │   ├── index.rs            # Índice local (Offset, Length, ShardID em disco)
│   │   │   └── scrubber.rs         # Background scanner contra corrupção silenciosa (Bitrot)
│   │   └── Cargo.toml
│   │
│   ├── z3s-metadata/               # Catálogo de Metadados & Consenso Distribuído
│   │   ├── src/
│   │   │   ├── raft/               # Implementação do consenso distribuído (OpenRaft)
│   │   │   ├── partition.rs        # Particionamento lexical de chaves (Key-Range Sharding)
│   │   │   ├── witness.rs          # Cache de validação Witness (Strong Read-After-Write)
│   │   │   ├── schema.rs           # Modelos de Bucket, Object, Part, Multipart
│   │   │   └── store.rs            # Interface do banco local LSM (RocksDB / Sled)
│   │   └── Cargo.toml
│   │
│   ├── z3s-auth/                   # Autenticação SigV4 & Políticas de Acesso
│   │   ├── src/
│   │   │   ├── sigv4.rs            # Parser e validador HMAC-SHA256 (Headers & Presigned URLs)
│   │   │   ├── streaming_sig.rs    # Validação de stream de chunks assinados
│   │   │   ├── iam.rs              # Motor de avaliação de políticas JSON (Allow/Deny/Conditions)
│   │   │   └── credentials.rs      # Provedor de AccessKey / SecretKey
│   │   └── Cargo.toml
│   │
│   ├── z3s-kms/                    # Criptografia em Repouso & Envelope Encryption
│   │   ├── src/
│   │   │   ├── sse_s3.rs           # AES-256-GCM com chaves mestras internas
│   │   │   ├── sse_c.rs            # Criptografia com chaves enviadas pelo cliente
│   │   │   ├── sse_kms.rs          # Envelope Encryption (DEK / KEK)
│   │   │   └── cipher.rs           # Aceleração criptográfica por hardware (AES-NI)
│   │   └── Cargo.toml
│   │
│   ├── z3s-cluster/                # Topologia de Rede & Placement Driver
│   │   ├── src/
│   │   │   ├── discovery.rs        # Descoberta de nós e Gossip Protocol
│   │   │   ├── topology.rs         # Mapa de domínios de falha (Racks, Zonas, Discos)
│   │   │   ├── placement.rs        # Algoritmo de distribuição ótima dos K+M shards
│   │   │   └── health.rs           # Phi-Accrual Failure Detector para nós inoperantes
│   │   └── Cargo.toml
│   │
│   └── z3s-gateway/                # Camada HTTP/REST S3 & Serialização
│       ├── src/
│       │   ├── server.rs           # Servidor HTTP/1.1 e HTTP/2 (Hyper/Tokio)
│       │   ├── router.rs           # Despachante de rotas S3
│       │   ├── handlers/           # Handlers de operações S3
│       │   │   ├── bucket.rs       # CreateBucket, DeleteBucket, ListBuckets
│       │   │   ├── object.rs       # PutObject, GetObject, DeleteObject, HeadObject
│       │   │   └── multipart.rs    # Initiate, UploadPart, Complete, Abort
│       │   ├── streaming.rs        # Pipeline de stream sem buffer excessivo em RAM
│       │   └── xml.rs              # Serializador/Deserializador XML compatível com AWS
│       └── Cargo.toml
│
└── tests/                          # Suítes de Testes de Integração & Caos
    ├── compliance/                 # Testes de compatibilidade com AWS CLI, Boto3, Go SDK
    ├── io_uring_bench/             # Benchmarks de throughput e IOPS de disco
    └── chaos/                      # Testes de partição de rede e queda forçada de nós
```

---

## 2. Responsabilidade Detalhada de Cada Crate

| Crate | Responsabilidade Principal |
| :--- | :--- |
| **`z3s-common`** | Estruturas de dados canônicas, wrappers de hash criptográfico/rápido (BLAKE3, HighwayHash, MD5, CRC32C) e mapeamento universal de erros HTTP/XML do S3. |
| **`z3s-erasure`** | Algoritmos de Reed-Solomon parametrizáveis ($K$ dados + $M$ paridade) acelerados por instruções vetoriais SIMD para fatiar e reconstruir streams de bytes. |
| **`z3s-storage`** | Gerenciador de disco em baixo nível (*ShardStore*). Escreve em arquivos de extensão pré-alocados com `io_uring`, gerencia o WAL local e executa o scrubber de bitrot. |
| **`z3s-metadata`** | Gerenciador do catálogo de índices e metadados. Coordena o consenso distribuído Raft, ordenação lexicográfica de chaves e o cache de validação Witness. |
| **`z3s-auth`** | Parser e validador de assinaturas AWS SigV4 e SigV2 (incluindo URLs pré-assinadas e streams assinados em tempo real) e motor de políticas IAM em JSON. |
| **`z3s-kms`** | Gerenciamento de chaves e criptografia em repouso (SSE-S3, SSE-C, SSE-KMS) com aceleração via AES-NI. |
| **`z3s-cluster`** | Descoberta dinâmica de nós, monitoramento de integridade da rede e algoritmo de posicionamento de shards respeitando domínios de falha físicos. |
| **`z3s-gateway`** | Servidor web HTTP REST compatível com S3, parser e serializador de esquemas XML oficiais, controle de fluxo de stream e rate-limiting. |

---

## 3. Fluxo de Execução Ponta a Ponta

### 3.1 Fluxo de Gravação (`PUT Object`)

```text
[Cliente S3 / SDK]
       │  1. HTTP PUT /meu-bucket/arquivo.bin (Headers SigV4, Content-Length)
       ▼
[z3s-gateway]
       │  2. Validação de autenticação via [z3s-auth] (SigV4 HMAC-SHA256)
       │  3. Verificação de permissões IAM / Bucket Policy
       │  4. Consulta [z3s-cluster] para obter nós de destino (Placement Driver)
       │
       ├──► [z3s-kms] Criptografa o stream se SSE estiver ativo
       │
       ├──► [z3s-erasure] Fatiamento do payload em K shards de dados + M shards de paridade
       │
       ▼  5. Envio simultâneo dos K+M shards em streaming assíncrono
[z3s-storage (Nós A..N)]
       │  6. Gravação nos Extent Files locais via io_uring + Direct I/O
       │  7. Gravação no WAL e cálculo do hash BLAKE3 do bloco
       │  8. Confirmação de escrita bem-sucedida de volta ao Gateway
       │
       ▼  9. Registro de Metadados
[z3s-metadata]
       │ 10. Gravação linearizável no cluster Raft (Bucket, Key, Size, ETag, Shard Pointers)
       │ 11. Atualização do Witness Cache para leitura estritamente consistente
       │
       ▼ 12. Resposta HTTP
[Cliente S3 / SDK] ◄── HTTP 200 OK (ETag: "9b10ea4073372f...")
```

### 3.2 Fluxo de Leitura (`GET Object` com Byte-Range)

```text
[Cliente S3 / SDK]
       │  1. HTTP GET /meu-bucket/arquivo.bin (Header: Range: bytes=1048576-2097151)
       ▼
[z3s-gateway]
       │  2. Validação SigV4 / Presigned URL via [z3s-auth]
       │  3. Consulta metadados e valida versão no [z3s-metadata] (Witness Cache)
       │  4. Localiza os ponteiros dos shards físicos correspondentes ao intervalo solicitado
       │
       ▼  5. Requisição paralela dos K shards necessários (sem precisar de todos os K+M)
[z3s-storage (Nós A..K)]
       │  6. Leitura precisa com offset no Extent File via io_uring
       │  7. Validação de integridade de hash on-the-fly
       │     (Se um shard falhar no hash, o gateway solicita um shard de paridade M para recuperar)
       │
       ▼  8. Reconstituição do stream original via [z3s-erasure] (se necessário decodificação)
[z3s-gateway]
       │  9. Descriptografia pelo [z3s-kms] (se objeto for criptografado)
       ▼ 10. Streaming HTTP dos bytes solicitados
[Cliente S3 / SDK] ◄── HTTP 206 Partial Content (Content-Range: bytes 1048576-2097151/52428800)
```

---

## 4. Modos de Operação e Implantação

1. **Modo All-in-One (`z3s-server`):**
   * Todos os subsistemas executam dentro do mesmo processo.
   * Ideal para ambientes de desenvolvimento local, testes automatizados e edge nodes compactos.
2. **Modo Distribuído em Escala:**
   * **Nós de Borda (`z3s-gateway`):** Totalmente stateless, escaláveis horizontalmente atrás de balanceadores L4/L7 (ex: Maglev, HAProxy, AWS NLB).
   * **Nós de Metadados (`z3s-metadata`):** Cluster de 3 ou 5 nós executando Raft em discos NVMe de alta velocidade.
   * **Nós de Armazenamento (`z3s-storaged`):** Servidores densos em armazenamento (ex: JBODs com dezenas de discos HDDs/SSDs) rodando o engine leve `z3s-storage`.
