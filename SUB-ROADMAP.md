#### 📌 FASE 0: Fundamentos Algorítmicos & Design do Layout em Disco

  Objetivo: Estabelecer o modelo matemático de armazenamento, layout de arquivos e tolerância a falhas antes de codificar a
  rede.

  • 0.1 Layout de Extents e Shards em Disco:
      • Desenhar o formato de armazenamento em bloco (Append-Only Extent Files de 64MB/128MB).
      • Evitar criar 1 arquivo por objeto no Linux (evita esgotamento de inodes e gargalo no ext4/XFS).
      • Mapeamento de (ShardID, Offset, Length, Checksum).
  • 0.2 Pipeline de Erasure Coding (Reed-Solomon):
      • Implementar partição em K blocos de dados e M de paridade com álgebra de Galois GF(2⁸).
      • Benchmarks de throughput com instruções SIMD/AVX2.
  • 0.3 Estrutura de Metadados e Checksums:
      • Definir o cabeçalho imutável do objeto e árvore de Merkle para uploads paralelos.

  ──────
  #### 📌 FASE 1: Core Engine de Armazenamento Local (Storage Node Daemon)

  Objetivo: Criar o daemon de nó de armazenamento responsável por persistir e ler blocos locais com o máximo de vazão do
  hardware.

  • 1.1 I/O com io_uring e Direct I/O:
      • Pipeline assíncrono de escrita com alinhamento de blocos em disco (4KB/4096 bytes para NVMe/SSDs).
      • Leitura com suporte a Byte-Range sem carregar o objeto inteiro na memória.
  • 1.2 Write-Ahead Log (WAL) e MemTable Local:
      • Implementar gravação atômica para garantir recuperação após crash repentino de energia.
  • 1.3 Verificação de Bitrot em Tempo Real:
      • Validação de hash no momento da leitura (on-the-fly verification).
      • Em caso de mismatch, sinalizar erro de I/O para recuperação via paridade.

  ──────
  #### 📌 FASE 2: S3 API Gateway & Autenticação

  Objetivo: Construir a camada de borda HTTP/REST 100% compatível com as ferramentas e SDKs oficiais da AWS (AWS CLI, Boto3,
  AWS SDK Go/JS).

  • 2.1 Implementação do Parser AWS SigV4 (Signature Version 4):
      • Autenticação HMAC-SHA256 em headers (Authorization: AWS4-HMAC-SHA256 ...) e via query params (Presigned URLs).
      • Suporte a AWS4-HMAC-SHA256-PAYLOAD e STREAMING-AWS4-HMAC-SHA256-PAYLOAD.
  • 2.2 Operações Básicas de Buckets e Objetos:
      • PUT /bucket/object (Streaming sem buffering total na RAM).
      • GET /bucket/object (Suporte a Range: bytes=start-end, If-Match, If-None-Match).
      • HEAD /bucket/object (Retorno ultra-rápido via camada de metadados).
      • DELETE /bucket/object.
  • 2.3 Serialização XML do S3:
      • Implementação rigorosa dos esquemas XML para ListBucketResult, ErrorResponse, InitiateMultipartUploadResult, etc.

  ──────
  #### 📌 FASE 3: Clusterização, Consenso & Metadados Distribuídos

  Objetivo: Transformar nós isolados em um cluster distribuído com alta disponibilidade e consistência forte.

  • 3.1 Cluster Coordinator & Topologia de Rede:
      • Descoberta de nós via Gossip Protocol ou etcd/Raft.
      • Failure Detector (baseado em algoritmos como Φ-Accural Failure Detector) para identificar nós caídos.
  • 3.2 Distributed Metadata Layer com Consistência Forte:
      • Particionamento de catálogo de chaves com sharding horizontal por prefixo.
      • Consenso distribuído linearizável com Raft.
      • Implementação do cache de metadados com validação Witness para zero leituras obsoletas (Read-after-write).
  • 3.3 Placement Driver (Distribuição de Dados):
      • Distribuição dos K + M fragmentos em Failure Domains distintos (racks, nós, controladoras de disco) para garantir
      durabilidade física.

  ──────
  #### 📌 FASE 4: Recursos Avançados do S3 (Multipart, Versionamento & List)

  Objetivo: Suportar objetos gigantes (até 5TB) e controle de versões de dados.

  • 4.1 Multipart Upload Engine:
      • InitiateMultipartUpload → geração de UploadId.
      • UploadPart → gravação concorrente de partes com cálculo de ETag individual.
      • CompleteMultipartUpload → unificação lógica de manifestos e cálculo de ETag composto (hash-partCount).
      • AbortMultipartUpload → liberação de recursos órfãos.
  • 4.2 Versionamento de Objetos:
      • Geração de VersionId (UUIDv7/Timestamp monótono).
      • Delete Markers para deleções lógicas com preservação do histórico de versões.
  • 4.3 List Objects v2 (GET /?list-type=2):
      • Suporte completo a paginação (ContinuationToken, MaxKeys), filtros de Prefix e agregação por Delimiter (/).

  ──────
  #### 📌 FASE 5: Segurança, Criptografia & Governança

  Objetivo: Garantir isolamento multi-tenant de nível corporativo e proteção de dados em trânsito e repouso.

  • 5.1 Criptografia em Repouso (SSE - Server-Side Encryption):
      • SSE-S3: Criptografia transparente com chaves gerenciadas pelo sistema (AES-256-GCM).
      • SSE-C: Chave fornecida pelo cliente no header HTTP (x-amz-server-side-encryption-customer-key).
      • SSE-KMS: Integração com Key Management Service interno/externo via Envelope Encryption (DEK - Data Encryption Key e
      KEK - Key Encryption Key).
  • 5.2 Mecanismo de Políticas IAM e Bucket Policies:
      • Motor de avaliação de políticas JSON no formato AWS (Effect, Principal, Action, Resource, Condition).
      • Suporte a ACLs legadas (private, public-read, authenticated-read).
  • 5.3 S3 Object Lock & WORM (Write Once, Read Many):
      • Bloqueio de retenção legal (Legal Hold) e retenção por tempo (Compliance/Governance Mode).

  ──────
  #### 📌 FASE 6: Resiliência, Garbage Collection & Operação em Grande Escala
  Objetivo: Operar continuamente sem intervenção manual mesmo sob falhas massivas de hardware.

  • 6.1 Background Bitrot Scrubber & Active Auto-Healing:
      • Scrubber contínuo percorrendo setores de disco com baixa prioridade de I/O.
      • Ao detectar corrupção de 1 shard, o motor baixa os shards sobreviventes, recalcula a matriz de Reed-Solomon e grava o
      shard restaurado em um novo disco disponível.
  • 6.2 Coletor de Lixo Distribuído (Garbage Collector & Compaction):
      • Limpeza de partes abandonadas de multipart uploads expirados.
      • Compactação de Extent Files para recuperar espaço de objetos deletados.
  • 6.3 Lifecycle Rules Engine:
      • Expiração automática de objetos e transição de classes de armazenamento (ex: Standard → Infrequent Access → Cold
      Archive).
  • 6.4 Testes de Caos & Validação Formal:
      • Injeção de partição de rede e morte de nós com suítes no estilo Jepsen e testes de concorrência com o model-checker
      Shuttle.

  ──────
  ### Recomendações de Próximos Passos Práticos

  1. Comece pelo MVP do Storage Node Local (Fase 0 e Fase 1): Valide o layout de gravação em disco e o cálculo de Reed-Solomon
  antes de construir a camada de rede.
  2. Defina a granularidade dos blocos: Um tamanho de chunk padrão entre 4MB e 8MB costuma ser o sweet spot entre overhead de
  metadados e performance de streaming de grandes arquivos.
  3. Use a suíte de testes de conformidade S3 (ex: s3-tests da comunidade Ceph): Isso garante que qualquer SDK oficial da AWS
  funcione no seu sistema desde o primeiro dia.