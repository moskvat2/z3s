use bytes::Bytes;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{error, info, warn};
use z3s_auth::sigv4::SigV4Engine;

pub const REPLICATION_HEADER: &str = "x-z3s-replication";

/// Configuração do Peer Secundário para Replicação Nativa
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReplicationConfig {
    pub peer_endpoint: String,
    pub peer_access_key: String,
    pub peer_secret_key: String,
    pub peer_region: String,
}

impl ReplicationConfig {
    pub fn new(
        peer_endpoint: String,
        peer_access_key: String,
        peer_secret_key: String,
        peer_region: Option<String>,
    ) -> Self {
        Self {
            peer_endpoint,
            peer_access_key,
            peer_secret_key,
            peer_region: peer_region.unwrap_or_else(|| "us-east-1".to_string()),
        }
    }
}

/// Payload recebido da interface web para atualizar a replicação
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReplicationUpdateRequest {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub peer_endpoint: Option<String>,
    #[serde(default)]
    pub peer_access_key: Option<String>,
    #[serde(default)]
    pub peer_secret_key: Option<String>,
    #[serde(default)]
    pub peer_region: Option<String>,
}

/// Eventos de mutação replicáveis
#[derive(Debug, Clone)]
pub enum ReplicationEvent {
    CreateBucket {
        bucket: String,
    },
    DeleteBucket {
        bucket: String,
    },
    PutObject {
        bucket: String,
        key: String,
        body: Bytes,
        content_type: String,
        metadata: HashMap<String, String>,
    },
    DeleteObject {
        bucket: String,
        key: String,
    },
}

/// Motor de Replicação Assíncrono Nativo
pub struct ReplicationEngine {
    pub config: ReplicationConfig,
    sender: mpsc::Sender<ReplicationEvent>,
}

impl ReplicationEngine {
    pub fn start(config: ReplicationConfig) -> Arc<Self> {
        let (sender, mut receiver) = mpsc::channel::<ReplicationEvent>(4096);
        let worker_config = config.clone();

        info!(
            "🔄 Iniciando Replication Worker nativo apontando para peer: {}",
            worker_config.peer_endpoint
        );

        let worker = async move {
            while let Some(event) = receiver.recv().await {
                let mut attempts = 0;
                let max_attempts = 3;
                let mut success = false;

                while attempts < max_attempts && !success {
                    attempts += 1;
                    match Self::dispatch_event(&worker_config, &event).await {
                        Ok(status)
                            if (status >= 200 && status < 300)
                                || (status == 409 && matches!(event, ReplicationEvent::CreateBucket { .. }))
                                || (status == 404
                                    && matches!(
                                        event,
                                        ReplicationEvent::DeleteBucket { .. }
                                            | ReplicationEvent::DeleteObject { .. }
                                    )) =>
                        {
                            info!("✅ Replicação com sucesso para peer (HTTP {})", status);
                            success = true;
                        }
                        Ok(status) => {
                            warn!(
                                "⚠️ Peer retornou HTTP {} ao replicar evento (tentativa {}/{})",
                                status, attempts, max_attempts
                            );
                            tokio::time::sleep(Duration::from_millis(200 * attempts as u64)).await;
                        }
                        Err(err) => {
                            warn!(
                                "⚠️ Erro de I/O ao replicar para peer: {} (tentativa {}/{})",
                                err, attempts, max_attempts
                            );
                            tokio::time::sleep(Duration::from_millis(200 * attempts as u64)).await;
                        }
                    }
                }

                if !success {
                    error!(
                        "❌ Falha definitiva ao replicar evento após {} tentativas",
                        max_attempts
                    );
                }
            }
        };

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(worker);
        } else {
            std::thread::Builder::new()
                .name("z3s-repl-worker".to_string())
                .spawn(move || {
                    if let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() {
                        rt.block_on(worker);
                    }
                })
                .ok();
        }

        Arc::new(Self { config, sender })
    }

    /// Enfileira um evento de forma não-bloqueante
    pub fn enqueue(&self, event: ReplicationEvent) {
        if let Err(e) = self.sender.try_send(event) {
            warn!("⚠️ Fila de replicação cheia ou fechada: {}", e);
        }
    }

    /// Testa a conectividade e autenticação com o peer
    pub async fn test_connectivity(config: &ReplicationConfig) -> Result<(), String> {
        match Self::send_http_sigv4(
            config,
            "GET",
            "/",
            Bytes::new(),
            "application/xml",
            HashMap::new(),
        ).await {
            Ok(code) if code == 200 || code == 204 => Ok(()),
            Ok(403) => Err("O peer remoto recusou a autenticação (HTTP 403 Forbidden). Verifique se o Access Key e Secret Key correspondem aos do peer.".to_string()),
            Ok(code) => Err(format!("Peer retornou resposta inesperada: HTTP {}", code)),
            Err(e) => Err(format!("Falha de conexão com o peer ({}): {}", config.peer_endpoint, e)),
        }
    }

    /// Dispara a requisição HTTP com assinatura AWS SigV4 para o peer
    async fn dispatch_event(
        config: &ReplicationConfig,
        event: &ReplicationEvent,
    ) -> Result<u16, Box<dyn std::error::Error + Send + Sync>> {
        let (method, path, body, content_type, extra_headers) = match event {
            ReplicationEvent::CreateBucket { bucket } => {
                ("PUT", format!("/{}", bucket), Bytes::new(), "application/xml", HashMap::new())
            }
            ReplicationEvent::DeleteBucket { bucket } => {
                ("DELETE", format!("/{}", bucket), Bytes::new(), "application/xml", HashMap::new())
            }
            ReplicationEvent::PutObject {
                bucket,
                key,
                body,
                content_type,
                metadata,
            } => {
                let path = format!("/{}", if key.starts_with('/') { format!("{}{}", bucket, key) } else { format!("{}/{}", bucket, key) });
                ("PUT", path, body.clone(), content_type.as_str(), metadata.clone())
            }
            ReplicationEvent::DeleteObject { bucket, key } => {
                let path = format!("/{}", if key.starts_with('/') { format!("{}{}", bucket, key) } else { format!("{}/{}", bucket, key) });
                ("DELETE", path, Bytes::new(), "application/xml", HashMap::new())
            }
        };

        Self::send_http_sigv4(config, method, &path, body, content_type, extra_headers).await
    }

    /// Envia uma requisição HTTP/1.1 assinada com AWS SigV4 para o peer
    pub async fn send_http_sigv4(
        config: &ReplicationConfig,
        method: &str,
        path: &str,
        body: Bytes,
        content_type: &str,
        extra_headers: HashMap<String, String>,
    ) -> Result<u16, Box<dyn std::error::Error + Send + Sync>> {
        // Parse endpoint host and port
        let ep = config.peer_endpoint.trim();
        let trimmed = ep
            .strip_prefix("http://")
            .or_else(|| ep.strip_prefix("https://"))
            .unwrap_or(ep);
        let host_port = trimmed.split('/').next().unwrap_or(trimmed);
        let host = if host_port.contains(':') {
            host_port.split(':').next().unwrap()
        } else {
            host_port
        };
        let port: u16 = if host_port.contains(':') {
            host_port.split(':').nth(1).unwrap().parse().unwrap_or(9000)
        } else {
            9000
        };

        let now = Utc::now();
        let timestamp = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date = now.format("%Y%m%d").to_string();

        let payload_hash = hex::encode(Sha256::digest(&body));

        let mut canonical_headers = BTreeMap::new();
        canonical_headers.insert("host".to_string(), host_port.to_string());
        canonical_headers.insert("x-amz-date".to_string(), timestamp.clone());
        canonical_headers.insert("x-amz-content-sha256".to_string(), payload_hash.clone());
        canonical_headers.insert(REPLICATION_HEADER.to_string(), "true".to_string());

        if !body.is_empty() || method == "PUT" {
            canonical_headers.insert("content-type".to_string(), content_type.to_string());
        }

        for (k, v) in extra_headers {
            canonical_headers.insert(k.to_ascii_lowercase(), v);
        }

        let signed_headers: Vec<String> = canonical_headers.keys().cloned().collect();

        let auth_header = SigV4Engine::generate_authorization_header(
            &config.peer_access_key,
            &config.peer_secret_key,
            &config.peer_region,
            "s3",
            method,
            path,
            "",
            &canonical_headers,
            &signed_headers,
            &payload_hash,
            &timestamp,
            &date,
        );

        let target_addr = format!("{}:{}", host, port);
        let mut stream = timeout(Duration::from_secs(5), TcpStream::connect(&target_addr)).await??;

        // Monta os cabeçalhos HTTP/1.1 no protocolo oficial de rede
        let mut raw_request = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\nContent-Length: {}\r\nAuthorization: {}\r\nConnection: close\r\n",
            method,
            path,
            host_port,
            body.len(),
            auth_header
        );

        for (k, v) in &canonical_headers {
            if k != "host" {
                raw_request.push_str(&format!("{}: {}\r\n", k, v));
            }
        }
        raw_request.push_str("\r\n");

        timeout(Duration::from_secs(15), stream.write_all(raw_request.as_bytes())).await??;
        if !body.is_empty() {
            timeout(Duration::from_secs(300), stream.write_all(&body)).await??;
        }
        stream.flush().await?;

        // Lê a resposta do peer
        let mut response_buf = [0u8; 1024];
        let bytes_read = timeout(Duration::from_secs(60), stream.read(&mut response_buf)).await??;
        let response_str = String::from_utf8_lossy(&response_buf[..bytes_read]);

        if let Some(first_line) = response_str.lines().next() {
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(code) = parts[1].parse::<u16>() {
                    return Ok(code);
                }
            }
        }

        Err("Resposta HTTP inválida recebida do peer".into())
    }
}
