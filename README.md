# Z3S — High-Performance S3-Compatible Object Storage

<p align="center">
  <strong>O sistema de armazenamento de objetos distribuído de alta performance desenvolvido em Rust, 100% compatível com a API AWS S3 e com Web Console embutido de zero dependência externa.</strong>
</p>

<p align="center">
  <a href="#-recursos-principais"><img src="https://img.shields.io/badge/Language-Rust%202021-orange.svg" alt="Rust 2021"></a>
  <a href="#-compatibilidade-com-aws-s3"><img src="https://img.shields.io/badge/AWS%20S3-SigV4%20Compatible-blue.svg" alt="AWS S3 Compatible"></a>
  <a href="#-performance--benchmarks"><img src="https://img.shields.io/badge/Latency%20p99-%3C%201.8ms-brightgreen.svg" alt="p99 Latency"></a>
  <a href="#-web-console-integrado"><img src="https://img.shields.io/badge/Web%20Console-Embedded%20SPA-blueviolet.svg" alt="Embedded Web Console"></a>
  <a href="#-resiliência--erasure-coding"><img src="https://img.shields.io/badge/Erasure%20Coding-Reed--Solomon%20SIMD-green.svg" alt="Reed-Solomon SIMD"></a>
</p>

---

## 📑 Índice

1. [Visão Geral](#-visão-geral)
2. [Recursos Principais](#-recursos-principais)
3. [Arquitetura do Sistema](#-arquitetura-do-sistema)
4. [Web Console Integrado](#-web-console-integrado)
5. [Compatibilidade com AWS S3](#-compatibilidade-com-aws-s3)
6. [Performance & Benchmarks de Estresse](#-performance--benchmarks-de-estresse)
7. [Início Rápido (Quickstart)](#-início-rápido-quickstart)
8. [Exemplos de Integração com Clientes](#-exemplos-de-integração-com-clientes)
   - [AWS CLI v2](#aws-cli-v2)
   - [Python (Boto3)](#python-boto3)
   - [Go (AWS SDK v2)](#go-aws-sdk-v2)
   - [Rust (aws-sdk-s3)](#rust-aws-sdk-s3)
9. [Segurança e Criptografia](#-segurança-e-criptografia)
10. [Disaster Recovery e Resiliência](#-disaster-recovery-e-resiliência)
11. [Documentação Completa do Projeto](#-documentação-completa-do-projeto)

---

## 🌟 Visão Geral

O **Z3S** é um sistema moderno de armazenamento de objetos desenvolvido em **Rust** para entregar a mais alta taxa de transferência (I/O throughput), latências previsíveis de submilisegundos e tolerância física a falhas em escala de petabytes.

Ao contrário de sistemas legados que sofrem com pausas de Garbage Collection (GC) da JVM ou Go runtime sob alta concorrência, o Z3S opera com gerenciamento de memória determinístico, chamadas de sistema I/O assíncronas de última geração (`io_uring`) e pipeline de codificação Reed-Solomon acelerado por hardware (AVX2, AVX-512 e ARM NEON).

---

## ⚡ Recursos Principais

- 🦀 **Zero Garbage Collection:** Desenvolvido 100% em Rust para latências p99 ultra-baixas e uso de memória constante.
- 🔐 **Autenticação AWS SigV4 / SigV2:** Compatibilidade total com chaves de acesso AWS, Presigned URLs e validação HMAC em tempo constante.
- 📁 **Engine de Extents Imutáveis:** Armazenamento append-only em blocos alinhados de 4096 bytes com varredura inteligente de cabeçalhos (*Zero Idle Disk I/O*).
- 🛡️ **Reed-Solomon SIMD ($K+M$):** Codificação de apagamento modular com regeneração autônoma de dados em caso de perda de discos.
- 🔍 **Bitrot Scrubber Contínuo:** Varredura em segundo plano para detecção proativa e autocura de corrupção silenciosa via hashes BLAKE3.
- 🌐 **Web Console Embutido:** Interface gráfica SPA completa (estilo AWS S3 Console) embutida diretamente no binário Rust, sem necessidade de Node.js, Nginx ou dependências externas em produção.
- 📦 **Multipart Upload de Alta Velocidade:** Suporte a upload concorrente de grandes arquivos com cálculo de ETag multipart composto.
- 🏷️ **Versionamento de Objetos & Object Lock:** Controle granular de versões históricas, marcadores de deleção e retenção WORM.
- 🔒 **Criptografia em Repouso (SSE):** Criptografia transparente SSE-S3 e SSE-KMS com AES-256-GCM via aceleração AES-NI.
- 📊 **Monitoramento e Métricas em Tempo Real:** Endpoints `/api/metrics` para acompanhamento de IOPS, RPS, Extents e integridade de nós.

---

## 🏗️ Arquitetura do Sistema

```mermaid
flowchart TD
    Clients["Aplicações / SDKs / AWS CLI / Web Browser"] -->|HTTP/REST (AWS SigV4) - Porta 9000| Gateway["Z3S S3 Gateway (Tokio / Hyper / Axum)"]

    subgraph GatewayCore["Gateway & Segurança"]
        Gateway --> Auth["Auth & IAM Engine (SigV4 / Policies / CORS)"]
        Gateway --> Router["S3 REST API Router & XML Serializer"]
        Gateway --> WebUI["Embedded Web Console (/console/)"]
    end

    subgraph CoreStorage["Z3S Storage Engine"]
        Router --> Meta["Metadata Catalog (Strong Consistency)"]
        Router --> Erasure["Reed-Solomon SIMD Encoder (K+M)"]
        Erasure --> ExtentEngine["Extent Engine (Aligned 4096B Blocks)"]
        ExtentEngine --> WAL["Write-Ahead Log (WAL - Crash Consistency)"]
    end

    subgraph PhysicalDisks["Camada Física de Armazenamento"]
        ExtentEngine --> E1[("Extent Files (.z3s)")]
        ExtentEngine --> E2[("Index / Shards (BLAKE3)")]
        ExtentEngine --> Scrubber["Background Bitrot Scrubber & Auto-Healing"]
    end
```

---

## 🖥️ Web Console Integrado

O Z3S inclui uma interface gráfica moderna inspirada no AWS S3 Management Console, acessível diretamente em `http://localhost:9000/console/`:

- **Navegador de Buckets e Objetos:** Criação com 1 clique, suporte a pastas virtuais, navegação por breadcrumbs e visualizador (*preview*) integrado de imagens, PDFs, códigos e vídeos.
- **Upload Drag-and-Drop:** Upload em lote com barra de progresso em tempo real e chaveamento automático para Multipart Upload em arquivos grandes.
- **Editor de Políticas de Bucket (JSON):** Editor com modelos pré-configurados (Public Read, IP Whitelist, Enforce HTTPS/SSE).
- **Editor de Regras CORS:** Configuração visual de origens permitidas, métodos HTTP e cabeçalhos expostos.
- **Gerenciador de Ciclo de Vida (Lifecycle):** Configuração de expiração de objetos e transições de retenção.
- **Gerenciador de Chaves IAM:** Criação e revogação dinâmica de `Access Key ID` e `Secret Access Key`.
- **Dashboard de Métricas em Tempo Real:** Uso de disco físico vs. lógico, total de extents, contagem de objetos e gráfico de operações.

---

## 📋 Compatibilidade com AWS S3

| Operação S3 | Suportada? | Descrição |
| :--- | :---: | :--- |
| `ListBuckets` / `ListAllMyBuckets` | ✅ | `GET /` com serialização XML oficial |
| `CreateBucket` / `DeleteBucket` | ✅ | `PUT /{bucket}` e `DELETE /{bucket}` |
| `PutObject` / `GetObject` | ✅ | Upload/Download com streaming, hashing BLAKE3 e SSE |
| `HeadObject` / `DeleteObject` | ✅ | Verificação de existência e deleção atômica |
| `ListObjectsV2` | ✅ | Suporte a `prefix`, `delimiter`, `max-keys` e `continuation-token` |
| `InitiateMultipartUpload` | ✅ | `POST /{bucket}/{key}?uploads` gerando `UploadId` |
| `UploadPart` | ✅ | `PUT /{bucket}/{key}?partNumber={N}&uploadId={ID}` |
| `CompleteMultipartUpload` | ✅ | `POST /{bucket}/{key}?uploadId={ID}` com validação de XML e ETag |
| `AbortMultipartUpload` | ✅ | `DELETE /{bucket}/{key}?uploadId={ID}` |
| `Get/PutBucketVersioning` | ✅ | Versionamento com identificadores UUIDv7 e Delete Markers |
| `Get/PutBucketPolicy` | ✅ | Avaliação de políticas JSON (`Effect`, `Action`, `Resource`) |
| `Get/PutBucketCors` | ✅ | Gestão de cabeçalhos CORS e pré-vôos HTTP OPTIONS |
| `Get/PutBucketLifecycleConfiguration` | ✅ | Regras de expiração e ciclo de vida |
| `Presigned URLs` | ✅ | Links temporários com expiração para upload e download |
| `Byte-Range Requests` | ✅ | Cabeçalho `Range: bytes=X-Y` com zero-copy I/O |

---

## 🚀 Performance & Benchmarks de Estresse

O Z3S foi submetido a testes agressivos de estresse e concorrência massiva:

- **Volume de Teste:** 10.000 requisições concorrentes de alta densidade (PUT, GET, LIST, DELETE).
- **Taxa de Erro:** **0.00%** de perda ou falha de requisições.
- **Latência p99:** **1.78 ms** em operações de leitura/escrita simultâneas.
- **Uso de Disco em Repouso:** **0.00 KB/s** de leitura/escrita graças à varredura otimizada de cabeçalhos (*Zero Idle Disk I/O*).
- **Recuperação de Falhas (DR):** RPO = 0 e RTO < 500ms em testes de failover e snapshot restore.

*(Para o relatório completo, consulte [STRESS-TEST-REPORT.md](STRESS-TEST-REPORT.md) e [DR-TEST-REPORT.md](DR-TEST-REPORT.md)).*

---

## ⚡ Início Rápido (Quickstart)

### Pré-requisitos
- Compilador **Rust 1.75+** e `cargo`.
- Linux (x86_64 / ARM64) ou macOS.

### 1. Clonar o Repositório
```bash
git clone https://github.com/moskvat2/z3s.git
cd z3s
```

### 2. Compilar em Modo Release
```bash
cargo build --release
```

### 3. Executar o Servidor
```bash
./target/release/z3s-server
```

Por padrão, o servidor iniciará em `http://127.0.0.1:9000` utilizando o diretório local `./data`.

### 4. Variáveis de Ambiente Suportadas

| Variável | Padrão | Descrição |
| :--- | :--- | :--- |
| `Z3S_BIND_ADDR` | `0.0.0.0:9000` | Endereço e porta de escuta do S3 Gateway |
| `Z3S_DATA_DIR` | `./data` | Diretório raiz para extents e metadados |
| `Z3S_ACCESS_KEY` | `admin` | Chave de acesso root inicial |
| `Z3S_SECRET_KEY` | `admin123456` | Chave secreta root inicial |
| `Z3S_ENABLE_ENCRYPTION` | `true` | Ativa criptografia SSE-S3 AES-256-GCM |
| `RUST_LOG` | `info` | Nível de verbosidade de logs (`debug`, `info`, `warn`) |

---

## 🔌 Exemplos de Integração com Clientes

### AWS CLI v2

Configure as credenciais:
```bash
export AWS_ACCESS_KEY_ID=admin
export AWS_SECRET_ACCESS_KEY=admin123456
export AWS_DEFAULT_REGION=us-east-1
```

Criar bucket e transferir arquivos:
```bash
# Criar bucket
aws --endpoint-url http://localhost:9000 s3 mb s3://meu-bucket

# Fazer upload de arquivo
aws --endpoint-url http://localhost:9000 s3 cp ./documento.pdf s3://meu-bucket/documento.pdf

# Listar objetos
aws --endpoint-url http://localhost:9000 s3 ls s3://meu-bucket/

# Baixar arquivo
aws --endpoint-url http://localhost:9000 s3 cp s3://meu-bucket/documento.pdf ./baixado.pdf
```

---

### Python (Boto3)

```python
import boto3
from botocore.client import Config

s3 = boto3.client(
    's3',
    endpoint_url='http://localhost:9000',
    aws_access_key_id='admin',
    aws_secret_access_key='admin123456',
    config=Config(signature_version='s3v4'),
    region_name='us-east-1'
)

# Criar bucket
s3.create_bucket(Bucket='dados-analytics')

# Upload de payload em memória
s3.put_object(
    Bucket='dados-analytics',
    Key='metricas/2026-09.json',
    Body=b'{"status": "ok", "throughput_mbs": 1250}'
)

# Download de objeto
obj = s3.get_object(Bucket='dados-analytics', Key='metricas/2026-09.json')
print(obj['Body'].read().decode('utf-8'))
```

---

### Go (AWS SDK v2)

```go
package main

import (
	"context"
	"fmt"
	"strings"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/credentials"
	"github.com/aws/aws-sdk-go-v2/service/s3"
)

func main() {
	r := strings.NewReader("Payload de alta velocidade gravado no Z3S")

	customResolver := aws.EndpointResolverWithOptionsFunc(func(service, region string, options ...interface{}) (aws.Endpoint, error) {
		return aws.Endpoint{
			URL:               "http://localhost:9000",
			SigningRegion:     "us-east-1",
			HostnameImmutable: true,
		}, nil
	})

	cfg, err := config.LoadDefaultConfig(context.TODO(),
		config.WithEndpointResolverWithOptions(customResolver),
		config.WithCredentialsProvider(credentials.NewStaticCredentialsProvider("admin", "admin123456", "")),
	)
	if err != nil {
		panic(err)
	}

	client := s3.NewFromConfig(cfg, func(o *s3.Options) {
		o.UsePathStyle = true
	})

	_, err = client.PutObject(context.TODO(), &s3.PutObjectInput{
		Bucket: aws.String("go-bucket"),
		Key:    aws.String("exemplo.txt"),
		Body:   r,
	})
	if err != nil {
		panic(err)
	}
	fmt.Println("Objeto gravado com sucesso no Z3S!")
}
```

---

### Rust (aws-sdk-s3)

```rust
use aws_sdk_s3::{config::Credentials, config::Region, Client};
use aws_types::sdk_config::SharedCredentialsProvider;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let creds = Credentials::new("admin", "admin123456", None, None, "static");
    let config = aws_sdk_s3::Config::builder()
        .credentials_provider(SharedCredentialsProvider::new(creds))
        .region(Region::new("us-east-1"))
        .endpoint_url("http://localhost:9000")
        .force_path_style(true)
        .build();

    let client = Client::from_conf(config);

    let resp = client.list_buckets().send().await?;
    for bucket in resp.buckets() {
        println!("Bucket encontrado: {:?}", bucket.name());
    }

    Ok(())
}
```

---

## 🔒 Segurança e Criptografia

- **Assinatura Criptográfica:** Implementação estrita de AWS SigV4 (HMAC-SHA256) resistente a ataques de timing (*Constant-time comparison*).
- **Criptografia em Repouso:** Criptografia SSE-S3 utilizando **AES-256-GCM** acelerado por hardware (*AES-NI*).
- **Controle de Acesso IAM & Políticas:** Avaliação de políticas de bucket baseadas em JSON com suporte a restrições de IP, ações permitidas e negações explícitas (*Explicit Deny*).
- **Block Public Access:** Proteção configurável por bucket para prevenir exposição não intencional de dados na web pública.

---

## 🛡️ Disaster Recovery e Resiliência

O Z3S possui módulos integrados de recuperação e proteção contra desastres:
- **Snapshot Manager:** Exportação e restauração atômica de instantes de tempo do catálogo de metadados e extents físicos.
- **Failover Zero-Loss:** Validação de transição de nós secundários em caso de interrupção do nó primário.
- **Scrubber de Integridade:** Varredura periódica em busca de corrupção silenciosa (*bitrot*) e autocura via paridade Reed-Solomon.

---

## 📚 Documentação Completa do Projeto

| Documento | Descrição |
| :--- | :--- |
| 🏗️ **[ARCHITECTURE.md](ARCHITECTURE.md)** | Estrutura modular de crates, fluxo de dados e design interno |
| 🗺️ **[ROADMAP.md](ROADMAP.md)** | Roadmap técnico consolidado das Fases 0 a 6 |
| 🌐 **[WEB-CONSOLE-ROADMAP.md](WEB-CONSOLE-ROADMAP.md)** | Especificação completa da interface gráfica embutida |
| 🏢 **[ENTERPRISE-ROADMAP.md](ENTERPRISE-ROADMAP.md)** | Roadmap de recursos Enterprise (WORM, S3 Select, CRR, POSIX FUSE) |
| 📖 **[MANUAL-DE-IMPLANTACAO.md](MANUAL-DE-IMPLANTACAO.md)** | Guia operacional para produção em bare-metal, OpenRC, Systemd e Docker |
| 🧪 **[STRESS-TEST-REPORT.md](STRESS-TEST-REPORT.md)** | Relatório de benchmark com 10.000 requisições sob estresse |
| 🛡️ **[DR-TEST-REPORT.md](DR-TEST-REPORT.md)** | Relatório de testes de Disaster Recovery e resiliência |
