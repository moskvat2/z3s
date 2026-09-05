# S3-Compatible Distributed Object Storage System — Architectural Blueprint & Roadmap

Este documento define a especificação arquitetural, a stack tecnológica e o roadmap de engenharia de ponta a ponta para o desenvolvimento do **Z3S** — um sistema de armazenamento de objetos compatível com a API AWS S3, projetado para ser leve, ultra-eficiente, seguro e escalável a petabytes/exabytes de dados.

---

## 1. Visão Geral & Princípios de Design

1. **Desacoplamento Total de Planos:** O *Data Path* (transferência de bytes de payload) e o *Metadata Path* (índice, nomes, autorização e catálogos) operam em subsistemas independentes.
2. **Zero Overhead de Runtime (Sem Garbage Collection):** Uso estrito de **Rust** para o núcleo de armazenamento, eliminando pausas de GC e garantindo latências p99 previsíveis em altas taxas de I/O.
3. **I/O Assíncrono com Kernel Moderno:** Utilização de `io_uring` e Direct I/O (`O_DIRECT`) para contornar gargalos de page cache do Linux e permitir streaming zero-copy.
4. **Resiliência Máxima & Bitrot Scrubbing:** Codificação de apagamento (*Erasure Coding*) via Reed-Solomon acelerado por instruções vetoriais (AVX-512 / AVX2 / ARM NEON) e verificação contínua de integridade via hashes rápidos (BLAKE3/HighwayHash).
5. **Consistência Estrita (*Strong Read-After-Write*):** Garantia de consistência forte para operações `PUT`, `DELETE` e `LIST` com modelo de validação via *Witness*.

---

## 2. Diagrama da Arquitetura do Sistema

```
                             [ Clientes / SDKs AWS ]
                    (AWS CLI, Boto3, AWS SDK Go/JS/Rust, etc.)
                                       │
                                       ▼ HTTPS (TLS 1.3 / SigV4)
┌─────────────────────────────────────────────────────────────────────────────┐
│                              S3 GATEWAY TIER                                │
│  • SigV4 / SigV2 Authenticator & Presigned URL Parser                       │
│  • REST S3 API Router (Buckets & Objects CRUD, Multipart, Byte-Ranges)       │
│  • Rate Limiting, CORS, XML Response Serializer                             │
└──────────────────────┬──────────────────────────────┬───────────────────────┘
                       │ (Metadata RPC)               │ (Streaming Shards)
                       ▼                              ▼
┌────────────────────────────────────────┐ ┌──────────────────────────────────┐
│         METADATA & INDEX TIER          │ │       DATA PLACEMENT TIER        │
│  • Distributed Raft Consensus          │ │  • Dynamic Shard Allocation     │
│  • Lexicographical Key Partitioning    │ │  • Failure Domain Awareness     │
│  • Witness Cache Validation            │ │    (Rack/Node/Disk level)        │
│  • Versioning & Object Lock (WORM)     │ │  • Storage Class Tiering         │
└────────────────────────────────────────┘ └────────────────┬─────────────────┘
                                                            │ (Internal Transport)
                                                            ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                       STORAGE NODE DAEMONS (ShardStore)                     │
│  ┌────────────────────────┐ ┌────────────────────────┐ ┌──────────────────┐ │
│  │ Node 1 (Disks A..N)    │ │ Node 2 (Disks A..N)    │ │ Node M (Disks..) │ │
│  │ • io_uring + Direct I/O│ │ • io_uring + Direct I/O│ │ • io_uring...    │ │
│  │ • Extent Engine (64MB) │ │ • Extent Engine (64MB) │ │ • Extent Engine  │ │
│  │ • Bitrot Scrubber      │ │ • Bitrot Scrubber      │ │ • Bitrot Scrubber│ │
│  │ • Local WAL & Index    │ │ • Local WAL & Index    │ │ • Local WAL      │ │
│  └────────────────────────┘ └────────────────────────┘ └──────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Stack Tecnológica de Alta Performance

| Subsistema | Tecnologia / Ferramenta | Motivo da Escolha |
| :--- | :--- | :--- |
| **Linguagem Principal** | **Rust** (Edição 2021+) | Segurança de memória sem runtime/GC, controle de baixo nível de ponteiros, compatibilidade com assembly/SIMD e performance pura. |
| **Async Runtime & Rede** | **Tokio + Hyper + Tower** | Arquitetura de micro-threads não-bloqueantes baseada em epoll/kqueue com alta vazão HTTP/1.1 e HTTP/2. |
| **I/O de Disco de Baixa Latência** | **Linux `io_uring` (`tokio-uring` / `io-uring` crate)** | Redução drástica de chamadas de sistema (syscalls) e cópias de contexto de kernel. |
| **Codificação de Apagamento (EC)** | **Intel ISA-L / `reed-solomon-simd`** | Cálculo de matrizes de Cauchy em $GF(2^8)$ com aceleração por hardware (vários GB/s por núcleo). |
| **TLS & Criptografia** | **Rustls + Ring / AWS-LC-RS** | Implementação moderna de TLS 1.3 imune a vulnerabilidades de OpenSSL, criptografia AES-256-GCM / ChaCha20 via hardware (AES-NI). |
| **Catálogo de Metadados** | **OpenRaft + LSM-Tree (RocksDB / Sled / Custom)** | Consenso distribuído linearizável com suporte a particionamento de índices e listagem lexical eficiente. |
| **Hashing & Checksums** | **BLAKE3 + HighwayHash + AVX-MD5 + CRC32C** | BLAKE3 para integridade interna ultra-rápida (>10 GB/s); MD5/CRC32C para compatibilidade com ETags dos SDKs S3. |
| **Observabilidade** | **OpenTelemetry + Prometheus + eBPF (Aya)** | Tracing distribuído de requisições, métricas de latência p99 de disco e diagnóstico profundo sem overhead. |

---

## 4. Roadmap Detalhado de Implementação (Fases 0 a 6)

### 📌 FASE 0: Fundamentos Algorítmicos & Layout em Disco
- [x] **0.1 Layout de Extents Físicos:**
  - Formato binário de arquivos de extensão (*Extent Files*) com cabeçalho alinhado de 4096 bytes (`Z3SE`) e cabeçalho de blocos de 64 bytes (`Z3SB`).
  - Estrutura de blocos: `[Magic 4B][ShardID 16B][ShardIndex 4B][TotalShards 4B][Length 4B][Checksum BLAKE3 32B]`.
  - Leitura e gravação com alinhamento de 4096 bytes e detecção ativa de corrupção silenciosa (*Bitrot*).
- [x] **0.2 Engine de Erasure Coding:**
  - Implementado motor modular Reed-Solomon $K+M$ acelerado por SIMD em `z3s-erasure`.
  - Algoritmo de reconstrução comprovado com tolerância a perda de até $M$ shards simultâneos.
- [x] **0.3 Estrutura de Metadados & Árvore de Merkle:**
  - Árvore de Merkle e geração de Provas de Inclusão (`MerkleProof`) com BLAKE3 para validação granular de chunks.
  - Modelos imutáveis `ObjectMetadata`, `ShardPointer` e `ObjectManifest`.

---

### 📌 FASE 1: Core Engine de Armazenamento Local (*Storage Node Daemon*)
- [x] **1.1 Daemon do Storage Node em Rust:**
  - Gerenciador de Extent Pool (`StorageEngine`) com rotação automática de arquivos Extent de 64MB/128MB.
  - Gravação atômica append-only com alinhamento de blocos de 4096 bytes.
- [x] **1.2 Write-Ahead Log (WAL) & MemTable Local:**
  - Garantia de *crash-consistency* com log binário estruturado com checksums CRC32C e recuperação automática pós-queda de energia.
- [x] **1.3 Suporte a Byte-Range I/O:**
  - Leitura posicional precisa (`read_shard_range`) com zero overhead de carregamento de bloco na memória RAM.
- [x] **1.4 Detecção de Bitrot & Background Scrubber:**
  - Validação de integridade de hash BLAKE3 em tempo real e varredura contínua em segundo plano (`BitrotScrubber`) com emissão de relatórios `ScrubReport`.

---

### 📌 FASE 2: S3 API Gateway & Autenticação
- [x] **2.1 Autenticação AWS SigV4 & SigV2:**
  - Parser e validador completo do cabeçalho `Authorization: AWS4-HMAC-SHA256 ...` e *Presigned URLs*.
  - Derivação de chaves HMAC em cascata (`kDate`, `kRegion`, `kService`, `kSigning`), construção de Canonical Request e String to Sign com comparação segura em tempo constante.
- [x] **2.2 Roteamento REST S3 Básico:**
  - `PUT /bucket` (CreateBucket) e `DELETE /bucket` (DeleteBucket).
  - `GET /` (ListBuckets) e `GET /bucket?list-type=2` (ListObjectsV2).
  - `PUT /bucket/object` (Upload via pipeline com codificação Reed-Solomon e gravação direta em disco).
  - `GET /bucket/object` (Download com suporte a `Range: bytes=X-Y` e recuperação de shards).
  - `HEAD /bucket/object` e `DELETE /bucket/object`.
- [x] **2.3 Serialização XML Padrão S3:**
  - Implementação das respostas oficiais da AWS para erros (`<Error>`), listagem de buckets (`<ListAllMyBucketsResult>`) e objetos (`<ListBucketResult>`).

---

### 📌 FASE 3: Clusterização, Consenso & Metadados Distribuídos
- [ ] **3.1 Cluster Coordinator & Topologia de Nós:**
  - Gerenciamento de adesão de nós (*Cluster Membership*) e detecção de falhas (*Heartbeats / Phi Accrual Failure Detector*).
  - Mapa de topologia distribuído (conhecimento de rack, zona e nó físico).
- [ ] **3.2 Camada de Metadados Distribuída com Consistência Forte:**
  - Replicação de metadados via protocolo de consenso **Raft**.
  - Particionamento horizontal de índices de chaves (*Key-Range Sharding*) em ordem lexicográfica.
  - Implementação do cache de metadados com validação *Witness* para garantir consistência imediata pós-escrita (*Read-After-Write*).
- [ ] **3.3 Placement Driver:**
  - Distribuição inteligente dos $K+M$ shards em nós fisicamente isolados para maximizar a durabilidade contra falhas de hardware.

---

### 📌 FASE 4: Recursos Avançados do S3
- [ ] **4.1 Multipart Upload Engine:**
  - `InitiateMultipartUpload` $\rightarrow$ Geração de `UploadId` único e registro temporário.
  - `UploadPart` $\rightarrow$ Gravação e hashing de partes independentes (5MB a 5GB por parte).
  - `CompleteMultipartUpload` $\rightarrow$ Validação de manifesto, cálculo de ETag composto (`<md5>-<partCount>`) e consolidação atômica de metadados.
  - `AbortMultipartUpload` $\rightarrow$ Cancelamento e liberação de shards órfãos.
- [ ] **4.2 Versionamento de Objetos:**
  - Atribuição de identificadores de versão ordenados no tempo (`VersionId` com UUIDv7/Timestamps monótonos).
  - Criação e tratamento de *Delete Markers* para deleções lógicas preservando versões anteriores.
- [ ] **4.3 List Objects v2 (`GET /?list-type=2`):**
  - Implementação de listagem de alta performance com `Prefix`, `Delimiter` (emulação de diretórios), `MaxKeys` e paginação com `ContinuationToken`.

---

### 📌 FASE 5: Segurança, Criptografia & Governança
- [ ] **5.1 Criptografia em Repouso (SSE - Server-Side Encryption):**
  - **SSE-S3:** Criptografia transparente com chaves mestras gerenciadas pelo sistema (AES-256-GCM).
  - **SSE-C:** Criptografia com chave fornecida pelo cliente no cabeçalho HTTP (`x-amz-server-side-encryption-customer-key`).
  - **SSE-KMS:** Criptografia de envelope (DEK e KEK) integrada com serviço de KMS interno ou HashiCorp Vault.
- [ ] **5.2 Mecanismo de Políticas IAM e Bucket Policies:**
  - Motor de avaliação de políticas JSON completas (`Effect`, `Principal`, `Action`, `Resource`, `Condition`).
  - Suporte a ACLs padrão (`private`, `public-read`, etc.).
- [ ] **5.3 S3 Object Lock & WORM (Write Once, Read Many):**
  - Retenção legal (*Legal Hold*) e retenção temporal (*Compliance/Governance Mode*).

---

### 📌 FASE 6: Resiliência, Garbage Collection & Escala Massiva
- [ ] **6.1 Background Bitrot Scrubber & Active Auto-Healing:**
  - Processo em segundo plano que inspeciona blocos de dados em repouso com prioridade I/O controlada.
  - Reconstrução autônoma: caso 1 ou mais discos falhem, o sistema utiliza os $K$ shards sobreviventes para regenerar os dados perdidos em novos discos.
- [ ] **6.2 Coletor de Lixo Distribuído (Garbage Collection & Compaction):**
  - Limpeza de partes abandonadas de multipart uploads expirados.
  - Compactação física e desfragmentação de *Extent Files* para recuperar espaço de objetos deletados.
- [ ] **6.3 Motor de Regras de Ciclo de Vida (*Lifecycle Policies*):**
  - Expiração automática de objetos e transição entre classes de armazenamento (*Hot/Standard* $\rightarrow$ *Warm* $\rightarrow$ *Cold*).
- [ ] **6.4 Testes de Caos & Validação:**
  - Execução de testes de conformidade utilizando a suíte `s3-tests`.
  - Injeção de partição de rede e falhas de disco sob carga contínua (estilo Jepsen).

---

## 5. Estrutura Recomendada do Repositório

```
z3s/
├── Cargo.toml                  # Workspace raiz Rust
├── ROADMAP.md                  # Especificação e roadmap do projeto
├── docs/                       # Documentação técnica e especificações
│   ├── architecture.md
│   └── s3-api-spec.md
├── crates/
│   ├── z3s-common/             # Tipos compartilhados, erros, hash e utilitários
│   ├── z3s-erasure/            # Implementação de Reed-Solomon SIMD
│   ├── z3s-storage-node/       # Engine de disco com io_uring, Extent Store e WAL
│   ├── z3s-metadata/           # Camada de metadados, Raft e Witness Cache
│   ├── z3s-auth/               # Parser SigV4/SigV2 e motor de políticas IAM
│   ├── z3s-gateway/            # Servidor HTTP/S3 REST e serializador XML
│   ├── z3s-kms/                # Envelope encryption e gerenciamento de chaves
│   └── z3s-cluster/            # Descoberta de nós, topologia e placement driver
└── tests/
    ├── s3_compliance/          # Testes com SDKs oficiais (boto3, aws-cli)
    └── chaos/                  # Testes de partição de rede e falhas de disco
```
