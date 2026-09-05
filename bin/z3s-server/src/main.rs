use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;
use z3s_auth::credentials::InMemoryCredentialsStore;
use z3s_erasure::ErasureEngine;
use z3s_gateway::{HttpServer, S3GatewayService};
use z3s_storage::{StorageEngine, DEFAULT_EXTENT_CAPACITY};

#[derive(Parser, Debug)]
#[command(name = "z3s-server", version, about = "Z3S S3-Compatible Distributed Object Storage Server")]
struct Cli {
    #[arg(short, long, default_value = "0.0.0.0:9000", env = "Z3S_BIND")]
    bind: String,

    #[arg(short, long, default_value = "./data", env = "Z3S_DATA_DIR")]
    data_dir: PathBuf,

    #[arg(long, default_value = "Z3SACCESSKEYEXAMPLE", env = "Z3S_ACCESS_KEY")]
    access_key: String,

    #[arg(long, default_value = "Z3SSECRETKEYEXAMPLE1234567890ABCDEF", env = "Z3S_SECRET_KEY")]
    secret_key: String,

    #[arg(long, default_value_t = 4, env = "Z3S_DATA_SHARDS")]
    data_shards: usize,

    #[arg(long, default_value_t = 2, env = "Z3S_PARITY_SHARDS")]
    parity_shards: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();
    let cli = Cli::parse();

    info!("============================================================");
    info!("🚀 Iniciando Z3S Object Storage Server (AWS S3 Compatible)");
    info!("📍 Endereço de Bind HTTP: http://{}", cli.bind);
    info!("🔑 Access Key ID:     {}", cli.access_key);
    info!("🔒 Secret Access Key: {}", cli.secret_key);
    info!("💾 Diretório de Dados Físicos: {:?}", cli.data_dir);
    info!("🛡️ Erasure Coding: {} dados + {} paridade (Reed-Solomon)", cli.data_shards, cli.parity_shards);
    info!("============================================================");

    // 1. Inicializa o motor de armazenamento em disco com Extents e WAL
    let storage = Arc::new(StorageEngine::open(&cli.data_dir, DEFAULT_EXTENT_CAPACITY)?);

    // 2. Inicializa o motor Reed-Solomon SIMD
    let erasure = Arc::new(ErasureEngine::new(cli.data_shards, cli.parity_shards)?);

    // 3. Registra as credenciais configuradas
    let credentials_store = Arc::new(InMemoryCredentialsStore::new());
    credentials_store.register(&cli.access_key, &cli.secret_key);
    // Registra aliases e credenciais padrões para desenvolvimento e compatibilidade
    credentials_store.register("z3sadmin", "z3sadminsecretkey");
    credentials_store.register("admin", "admin123456");
    credentials_store.register("root", "root123456");
    credentials_store.register("minioadmin", "minioadmin");

    // 4. Cria o Gateway Service com persistência de metadados de catálogo
    let node_id = Uuid::new_v4();
    let metadata_dir = cli.data_dir.join("metadata");
    let gateway_service = Arc::new(S3GatewayService::new_with_metadata(
        node_id,
        storage,
        erasure,
        credentials_store,
        Some(metadata_dir),
    ));

    // 5. Inicia o Garbage Collector & Extent Compactor em segundo plano (spawn_blocking a cada 30s)
    let gc_service = gateway_service.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let svc = gc_service.clone();
            let _ = tokio::task::spawn_blocking(move || {
                if let Ok(report) = svc.run_garbage_collection() {
                    if report.unreferenced_shards_deleted > 0 || report.extents_compacted > 0 || report.bytes_reclaimed > 0 {
                        info!(
                            "🧹 Storage GC/Compactor: {} shards limpos, {} extents reciclados, {} bytes liberados fisicamente no disco",
                            report.unreferenced_shards_deleted,
                            report.extents_compacted,
                            report.bytes_reclaimed
                        );
                    }
                }
            }).await;
        }
    });

    // 6. Inicia o servidor HTTP TCP
    let addr: SocketAddr = cli.bind.parse()?;
    let server = HttpServer::new(gateway_service);

    server.run(addr).await?;

    Ok(())
}
