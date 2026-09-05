# 🧪 AWS CLI Official Validation Roadmap — Z3S Storage

Este documento estabelece o plano rigoroso de homologação e validação ponta a ponta do **Z3S Object Storage** utilizando exclusivamente o cliente oficial **AWS CLI v2**.

---

## 📋 Matriz de Baterias de Teste

| Bateria | Escopo | Comandos AWS CLI | Status |
| :--- | :--- | :--- | :---: |
| **Bateria 1** | Ciclo de Vida e Operações de Buckets | `s3 mb`, `s3 ls`, `s3api head-bucket`, `s3 rb` | ✅ **100% Aprovado (4/4)** |
| **Bateria 2** | Operações Básicas de Objetos (CRUD) | `s3 cp`, `s3 mv`, `s3api head-object`, `s3 rm`, `s3api delete-objects` | ✅ **100% Aprovado (5/5)** |
| **Bateria 3** | Pastas Virtuais e Paginação (ListObjectsV2) | `s3 ls --recursive`, `s3api list-objects-v2 --delimiter --prefix --max-items` | ✅ **100% Aprovado (3/3)** |
| **Bateria 4** | Versionamento e Delete Markers | `s3api put-bucket-versioning`, `list-object-versions`, `get-object --version-id` | ✅ **100% Aprovado (3/3)** |
| **Bateria 5** | Multipart Uploads (Streaming de Grandes Volumes) | `s3 cp` (High Throughput 20MB+), `s3api create/upload-part/list/complete/abort` | ✅ **100% Aprovado (1/1)** |
| **Bateria 6** | Segurança, Criptografia (SSE) e Políticas | `s3api put/get/delete-bucket-encryption`, `put/get/delete-bucket-policy` | ✅ **100% Aprovado (2/2)** |
| **Bateria 7** | Regras de Ciclo de Vida (Lifecycle) | `s3api put/get/delete-bucket-lifecycle-configuration` | ✅ **100% Aprovado (1/1)** |
| **Bateria 8** | Byte-Range Requests & Integridade de Streaming | `s3api get-object --range "bytes=..."` com validação de hashes BLAKE3/MD5 | ✅ **100% Aprovado (1/1)** |

---

## 🎯 Detalhamento dos Casos de Teste

### 1. Bateria 1: Buckets
- `1.1`: Criação de buckets com nomes convencionais (`s3 mb s3://teste-bucket-01`).
- `1.2`: Listagem global de buckets do proprietário (`s3 ls`).
- `1.3`: Checagem de existência e cabeçalhos de bucket (`s3api head-bucket`).
- `1.4`: Remoção de bucket vazio (`s3 rb s3://...`).
- `1.5`: Rejeição adequada ao tentar criar bucket com nome duplicado ou inválido.

### 2. Bateria 2: Operações Básicas de Objetos
- `2.1`: Upload simples de arquivos de texto, binários aleatórios e arquivos de 0 bytes (vazios).
- `2.2`: Download de objetos e verificação de integridade bit a bit (`diff` / `md5sum`).
- `2.3`: Leitura de cabeçalhos e metadados (`s3api head-object`).
- `2.4`: Cópia interna de objetos (`s3 cp s3://... s3://...`).
- `2.5`: Exclusão individual (`s3 rm`) e em lote via XML/JSON (`s3api delete-objects`).

### 3. Bateria 3: Pastas Virtuais e ListObjectsV2
- `3.1`: Criação de hierarquias aninhadas (`nivel1/nivel2/nivel3/arquivo.txt`).
- `3.2`: Listagem filtrada por prefixo (`--prefix "nivel1/nivel2/"`).
- `3.3`: Agregação por delimitador de pasta (`--delimiter "/"` gerando `CommonPrefixes`).
- `3.4`: Paginação com tokens de continuação (`--max-items` e `--starting-token`).

### 4. Bateria 4: Versionamento de Objetos
- `4.1`: Habilitação de versionamento (`Status=Enabled`).
- `4.2`: Sobrescrita sucessiva gerando histórico de `VersionId` (UUIDv7).
- `4.3`: Consulta de versões completas (`list-object-versions`).
- `4.4`: Exclusão lógica criando `DeleteMarker` como versão atual.
- `4.5`: Recuperação de versão histórica específica (`get-object --version-id`).
- `4.6`: Exclusão definitiva de versão específica (`delete-object --version-id`).

### 5. Bateria 5: Multipart Uploads
- `5.1`: Upload automático em streaming de grandes volumes (>20MB) com divisão paralela de partes.
- `5.2`: Fluxo manual detalhado via `s3api`: `create-multipart-upload` -> `upload-part` -> `list-parts` -> `complete-multipart-upload`.
- `5.3`: Validação de ETag de multipart (`hash-partCount`).
- `5.4`: Cancelamento com `abort-multipart-upload` e limpeza de fragmentos órfãos no disco.

### 6. Bateria 6: Segurança e Criptografia (KMS & SSE)
- `6.1`: Definição de criptografia padrão no bucket (SSE-S3 `AES256`).
- `6.2`: Upload com headers explícitos de SSE (`--sse AES256` e `--sse aws:kms`).
- `6.3`: Download com decriptografia transparente sem corrupção dos dados.
- `6.4`: Definição, consulta e remoção de Políticas de Bucket (JSON Policy).

### 7. Bateria 7: Ciclo de Vida (Lifecycle Engine)
- `7.1`: Aplicação de regras de expiração por idade (`Days`) e prefixo.
- `7.2`: Aplicação de transição de classe de armazenamento (`GLACIER` / `STANDARD_IA`).
- `7.3`: Limpeza programada de multipart uploads incompletos.
- `7.4`: Consulta e remoção de configurações de ciclo de vida.

### 8. Bateria 8: Byte-Range Requests & Streaming
- `8.1`: Solicitação de faixas de bytes específicas (`bytes=0-100`, `bytes=500-1000`).
- `8.2`: Validação da resposta HTTP 206 Partial Content e integridade exata do corte.
