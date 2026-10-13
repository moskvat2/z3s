use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;
use std::time::Instant;

/// Coletor centralizado de métricas de alta performance para observabilidade padrão Prometheus
pub struct MetricsCollector {
    start_time: Instant,
    requests_total: RwLock<BTreeMap<(String, u16), u64>>,
    bytes_in_total: AtomicU64,
    bytes_out_total: AtomicU64,
    replication_success_total: AtomicU64,
    replication_failure_total: AtomicU64,
    replication_queued_total: AtomicU64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            requests_total: RwLock::new(BTreeMap::new()),
            bytes_in_total: AtomicU64::new(0),
            bytes_out_total: AtomicU64::new(0),
            replication_success_total: AtomicU64::new(0),
            replication_failure_total: AtomicU64::new(0),
            replication_queued_total: AtomicU64::new(0),
        }
    }

    /// Retorna o tempo de atividade do servidor em segundos
    pub fn uptime_seconds(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    /// Registra uma requisição HTTP atendida
    pub fn record_request(&self, method: &str, status: u16, bytes_in: usize, bytes_out: usize) {
        {
            if let Ok(mut guard) = self.requests_total.write() {
                *guard.entry((method.to_ascii_uppercase(), status)).or_insert(0) += 1;
            }
        }
        self.bytes_in_total.fetch_add(bytes_in as u64, Ordering::Relaxed);
        self.bytes_out_total.fetch_add(bytes_out as u64, Ordering::Relaxed);
    }

    /// Registra um evento de replicação
    pub fn record_replication(&self, status: &str) {
        match status {
            "success" => {
                self.replication_success_total.fetch_add(1, Ordering::Relaxed);
            }
            "failure" => {
                self.replication_failure_total.fetch_add(1, Ordering::Relaxed);
            }
            "queued" => {
                self.replication_queued_total.fetch_add(1, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    /// Renderiza as métricas no formato padrão de exportação do Prometheus / OpenMetrics
    pub fn render_prometheus(
        &self,
        buckets_count: usize,
        objects_count: usize,
        logical_bytes: u64,
        physical_bytes: u64,
        extents_count: usize,
        outbox_pending: usize,
    ) -> String {
        let mut out = String::with_capacity(2048);

        out.push_str("# HELP z3s_server_uptime_seconds Servidor Z3S uptime em segundos\n");
        out.push_str("# TYPE z3s_server_uptime_seconds gauge\n");
        out.push_str(&format!("z3s_server_uptime_seconds {:.2}\n\n", self.uptime_seconds()));

        out.push_str("# HELP z3s_http_requests_total Total de requisicoes HTTP atendidas\n");
        out.push_str("# TYPE z3s_http_requests_total counter\n");
        {
            let guard = self.requests_total.read().unwrap();
            if guard.is_empty() {
                out.push_str("z3s_http_requests_total{method=\"GET\",status=\"200\"} 0\n");
            } else {
                for ((method, status), count) in guard.iter() {
                    out.push_str(&format!(
                        "z3s_http_requests_total{{method=\"{}\",status=\"{}\"}} {}\n",
                        method, status, count
                    ));
                }
            }
        }
        out.push('\n');

        out.push_str("# HELP z3s_http_bytes_received_total Total de bytes recebidos via HTTP (ingress)\n");
        out.push_str("# TYPE z3s_http_bytes_received_total counter\n");
        out.push_str(&format!("z3s_http_bytes_received_total {}\n\n", self.bytes_in_total.load(Ordering::Relaxed)));

        out.push_str("# HELP z3s_http_bytes_sent_total Total de bytes enviados via HTTP (egress)\n");
        out.push_str("# TYPE z3s_http_bytes_sent_total counter\n");
        out.push_str(&format!("z3s_http_bytes_sent_total {}\n\n", self.bytes_out_total.load(Ordering::Relaxed)));

        out.push_str("# HELP z3s_buckets_total Total de buckets criados\n");
        out.push_str("# TYPE z3s_buckets_total gauge\n");
        out.push_str(&format!("z3s_buckets_total {}\n\n", buckets_count));

        out.push_str("# HELP z3s_objects_total Total de objetos armazenados\n");
        out.push_str("# TYPE z3s_objects_total gauge\n");
        out.push_str(&format!("z3s_objects_total {}\n\n", objects_count));

        out.push_str("# HELP z3s_storage_logical_bytes Tamanho logico total dos objetos armazenados\n");
        out.push_str("# TYPE z3s_storage_logical_bytes gauge\n");
        out.push_str(&format!("z3s_storage_logical_bytes {}\n\n", logical_bytes));

        out.push_str("# HELP z3s_storage_physical_bytes Tamanho fisico total ocupado pelos extents em disco\n");
        out.push_str("# TYPE z3s_storage_physical_bytes gauge\n");
        out.push_str(&format!("z3s_storage_physical_bytes {}\n\n", physical_bytes));

        out.push_str("# HELP z3s_storage_extents_total Quantidade de arquivos de extents ativos\n");
        out.push_str("# TYPE z3s_storage_extents_total gauge\n");
        out.push_str(&format!("z3s_storage_extents_total {}\n\n", extents_count));

        out.push_str("# HELP z3s_replication_events_total Total de eventos de replicacao nativa\n");
        out.push_str("# TYPE z3s_replication_events_total counter\n");
        out.push_str(&format!(
            "z3s_replication_events_total{{status=\"success\"}} {}\n",
            self.replication_success_total.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "z3s_replication_events_total{{status=\"failure\"}} {}\n",
            self.replication_failure_total.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "z3s_replication_events_total{{status=\"queued\"}} {}\n\n",
            self.replication_queued_total.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP z3s_replication_queue_pending Quantidade de eventos na fila persistente de outbox\n");
        out.push_str("# TYPE z3s_replication_queue_pending gauge\n");
        out.push_str(&format!("z3s_replication_queue_pending {}\n", outbox_pending));

        out
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_render() {
        let collector = MetricsCollector::new();
        collector.record_request("GET", 200, 100, 200);
        collector.record_request("PUT", 200, 4096, 50);
        collector.record_request("GET", 404, 10, 80);
        collector.record_replication("success");
        collector.record_replication("queued");

        let rendered = collector.render_prometheus(3, 50, 204800, 524288, 1, 0);
        assert!(rendered.contains("z3s_server_uptime_seconds"));
        assert!(rendered.contains("z3s_http_requests_total{method=\"GET\",status=\"200\"} 1"));
        assert!(rendered.contains("z3s_http_requests_total{method=\"PUT\",status=\"200\"} 1"));
        assert!(rendered.contains("z3s_http_requests_total{method=\"GET\",status=\"404\"} 1"));
        assert!(rendered.contains("z3s_buckets_total 3"));
        assert!(rendered.contains("z3s_objects_total 50"));
        assert!(rendered.contains("z3s_storage_logical_bytes 204800"));
        assert!(rendered.contains("z3s_storage_physical_bytes 524288"));
        assert!(rendered.contains("z3s_replication_events_total{status=\"success\"} 1"));
        assert!(rendered.contains("z3s_replication_events_total{status=\"queued\"} 1"));
        assert!(rendered.contains("z3s_replication_queue_pending 0"));
    }
}
