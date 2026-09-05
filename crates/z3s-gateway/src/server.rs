use crate::service::S3GatewayService;
use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

pub type BoxedBody = BoxBody<Bytes, hyper::Error>;

/// Servidor HTTP assíncrono para expor o Gateway S3 na rede
pub struct HttpServer {
    service: Arc<S3GatewayService>,
}

impl HttpServer {
    pub fn new(service: Arc<S3GatewayService>) -> Self {
        Self { service }
    }

    /// Inicia o listener TCP e atende requisições HTTP/1.1 de forma não-bloqueante
    pub async fn run(&self, addr: SocketAddr) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(addr).await?;
        info!("🚀 Z3S Gateway Server escutando em http://{}", addr);

        loop {
            let (stream, remote_addr) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let service = self.service.clone();

            tokio::task::spawn(async move {
                let service_fn_handler = service_fn(move |req: Request<hyper::body::Incoming>| {
                    let service = service.clone();
                    async move {
                        let (parts, incoming_body) = req.into_parts();

                        // Coleta o corpo completo da requisição
                        let body_bytes = match incoming_body.collect().await {
                            Ok(collected) => collected.to_bytes(),
                            Err(e) => {
                                error!("Erro ao ler corpo da requisição: {}", e);
                                Bytes::new()
                            }
                        };

                        // Extrai headers em formato case-insensitive
                        let mut headers = HashMap::new();
                        for (k, v) in parts.headers.iter() {
                            if let Ok(v_str) = v.to_str() {
                                headers.insert(k.as_str().to_ascii_lowercase(), v_str.to_string());
                            }
                        }

                        let method = parts.method.as_str();
                        let path = parts.uri.path();
                        let query = parts.uri.query();

                        let gateway_resp = service.handle_request(method, path, query, &headers, &body_bytes);
                        info!("📥 HTTP [{}] {} (query: {:?}) -> status {}", method, path, query, gateway_resp.status);

                        let mut builder = Response::builder().status(
                            StatusCode::from_u16(gateway_resp.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                        );

                        // Garante que o Content-Length correto seja sempre enviado
                        let body_len = gateway_resp.body.len();
                        let mut has_content_length = false;

                        for (k, v) in &gateway_resp.headers {
                            if k.eq_ignore_ascii_case("content-length") {
                                has_content_length = true;
                            }
                            builder = builder.header(k, v);
                        }

                        if !has_content_length {
                            builder = builder.header("content-length", body_len.to_string());
                        }

                        // Headers padrões essenciais do ecossistema AWS S3
                        builder = builder
                            .header("server", "Z3S/1.0")
                            .header("x-amz-request-id", uuid::Uuid::new_v4().to_string());

                        let body: BoxedBody = Full::new(gateway_resp.body)
                            .map_err(|e| match e {})
                            .boxed();

                        Ok::<_, hyper::Error>(builder.body(body).unwrap())
                    }
                });

                if let Err(err) = http1::Builder::new().serve_connection(io, service_fn_handler).await {
                    error!("Erro ao servir conexão de {}: {:?}", remote_addr, err);
                }
            });
        }
    }
}
