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

fn serialize_bytes_hex<S>(bytes: &Bytes, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&hex::encode(bytes))
}

fn deserialize_bytes_hex<'de, D>(deserializer: D) -> Result<Bytes, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let s = String::deserialize(deserializer)?;
    let decoded = hex::decode(&s).map_err(serde::de::Error::custom)?;
    Ok(Bytes::from(decoded))
}

/// Eventos de mutação replicáveis com suporte a serialização para outbox persistente
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
        #[serde(serialize_with = "serialize_bytes_hex", deserialize_with = "deserialize_bytes_hex")]
        body: Bytes,
        content_type: String,
        metadata: HashMap<String, String>,
    },
    DeleteObject {
        bucket: String,
        key: String,
    },
}

/// Motor de Replicação Assíncrono Nativo com Fila Offline Persistente (Outbox)
pub struct ReplicationEngine {
    pub config: ReplicationConfig,
    sender: mpsc::Sender<ReplicationEvent>,
    outbox_dir: Option<std::path::PathBuf>,
}

impl ReplicationEngine {
    pub fn start(config: ReplicationConfig) -> Arc<Self> {
        Self::start_with_outbox(config, None)
    }

    pub fn start_with_outbox(config: ReplicationConfig, outbox_dir: Option<std::path::PathBuf>) -> Arc<Self> {
        if let Some(ref dir) = outbox_dir {
            let _ = std::fs::create_dir_all(dir);
        }

        let (sender, mut receiver) = mpsc::channel::<ReplicationEvent>(4096);
        let worker_config = config.clone();
        let worker_outbox = outbox_dir.clone();

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
                        "❌ Falha ao replicar evento após {} tentativas. Salvando na outbox persistente...",
                        max_attempts
                    );
                    if let Some(ref dir) = worker_outbox {
                        Self::save_to_outbox(dir, &event);
                    }
                }
            }
        };

        // Worker em segundo plano para drenar e re-sincronizar eventos da outbox quando o peer estiver online
        let resync_config = config.clone();
        let resync_outbox = outbox_dir.clone();
        let resync_worker = async move {
            let dir = match resync_outbox {
                Some(d) => d,
                None => return,
            };
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;

                let mut event_files = Vec::new();
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|ext| ext.to_str()) == Some("event") {
                            event_files.push(path);
                        }
                    }
                }
                if event_files.is_empty() {
                    continue;
                }
                event_files.sort();

                // Se houver eventos pendentes, testa conectividade com o peer primeiro
                if Self::test_connectivity(&resync_config).await.is_err() {
                    continue;
                }

                for path in event_files {
                    if let Ok(content) = std::fs::read(&path) {
                        if let Ok(event) = serde_json::from_slice::<ReplicationEvent>(&content) {
                            match Self::dispatch_event(&resync_config, &event).await {
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
                                    let _ = std::fs::remove_file(&path);
                                    info!(
                                        "✅ Evento pendente em outbox {:?} replicado com sucesso para peer (HTTP {})",
                                        path.file_name().unwrap_or_default(),
                                        status
                                    );
                                }
                                _ => {
                                    break;
                                }
                            }
                        } else {
                            let _ = std::fs::remove_file(&path);
                        }
                    }
                }
            }
        };

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(worker);
            handle.spawn(resync_worker);
        } else {
            let w_fut = worker;
            let r_fut = resync_worker;
            std::thread::Builder::new()
                .name("z3s-repl-worker".to_string())
                .spawn(move || {
                    if let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() {
                        rt.spawn(r_fut);
                        rt.block_on(w_fut);
                    }
                })
                .ok();
        }

        Arc::new(Self {
            config,
            sender,
            outbox_dir,
        })
    }

    /// Grava um evento de replicação de forma crash-safe no diretório de outbox
    pub fn save_to_outbox(outbox_dir: &std::path::Path, event: &ReplicationEvent) {
        let _ = std::fs::create_dir_all(outbox_dir);
        let now_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let id = uuid::Uuid::new_v4();
        let final_path = outbox_dir.join(format!("{:020}_{}.event", now_nanos, id));
        let tmp_path = outbox_dir.join(format!(".tmp_{:020}_{}", now_nanos, id));

        if let Ok(json) = serde_json::to_vec(event) {
            if std::fs::write(&tmp_path, &json).is_ok() {
                let _ = std::fs::rename(&tmp_path, &final_path);
                warn!(
                    "📦 Evento de replicação persistido na outbox offline: {:?}",
                    final_path.file_name().unwrap_or_default()
                );
            }
        }
    }

    /// Retorna a quantidade de eventos pendentes na fila física da outbox
    pub fn get_pending_outbox_count(&self) -> usize {
        if let Some(ref dir) = self.outbox_dir {
            if let Ok(entries) = std::fs::read_dir(dir) {
                return entries
                    .flatten()
                    .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("event"))
                    .count();
            }
        }
        0
    }

    /// Enfileira um evento de forma não-bloqueante; se a fila estiver cheia, persiste na outbox
    pub fn enqueue(&self, event: ReplicationEvent) {
        if let Err(e) = self.sender.try_send(event.clone()) {
            warn!("⚠️ Canal de replicação saturado ({}), gravando diretamente na outbox", e);
            if let Some(ref dir) = self.outbox_dir {
                Self::save_to_outbox(dir, &event);
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replication_event_serialization_roundtrip() {
        let mut meta = HashMap::new();
        meta.insert("x-amz-meta-custom".to_string(), "hello".to_string());
        let event = ReplicationEvent::PutObject {
            bucket: "my-bucket".to_string(),
            key: "folder/file.bin".to_string(),
            body: Bytes::from_static(b"HELLO REPLICATION WORLD 123456"),
            content_type: "application/octet-stream".to_string(),
            metadata: meta,
        };

        let json = serde_json::to_vec(&event).expect("serialize event");
        let decoded: ReplicationEvent = serde_json::from_slice(&json).expect("deserialize event");
        assert_eq!(event, decoded);
    }

    #[test]
    fn test_replication_outbox_save_and_count() {
        let temp = tempfile::tempdir().unwrap();
        let outbox_dir = temp.path().join("outbox");
        let event = ReplicationEvent::CreateBucket {
            bucket: "test-bucket".to_string(),
        };

        ReplicationEngine::save_to_outbox(&outbox_dir, &event);
        ReplicationEngine::save_to_outbox(&outbox_dir, &event);

        let cfg = ReplicationConfig::new(
            "http://127.0.0.1:9001".to_string(),
            "key".to_string(),
            "secret".to_string(),
            None,
        );
        let engine = ReplicationEngine::start_with_outbox(cfg, Some(outbox_dir.clone()));
        assert_eq!(engine.get_pending_outbox_count(), 2);
    }
}
