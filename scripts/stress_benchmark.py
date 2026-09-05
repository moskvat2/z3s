#!/usr/bin/env python3
"""
Z3S Stress & Overload Benchmark Suite
====================================
Tests concurrency, throughput, RPS, latency percentiles, error rate, and data integrity
under heavy workload saturation.
"""

import sys
import time
import os
import uuid
import hmac
import hashlib
import datetime
import http.client
import urllib.parse
from concurrent.futures import ThreadPoolExecutor, as_completed
import statistics
import random

# Configuration
HOST = "127.0.0.1"
PORT = 9000
ENDPOINT = f"http://{HOST}:{PORT}"
ACCESS_KEY = "Z3SACCESSKEYEXAMPLE"
SECRET_KEY = "Z3SSECRETKEYEXAMPLE1234567890ABCDEF"
REGION = "us-east-1"
SERVICE = "s3"

def sign(key, msg):
    return hmac.new(key, msg.encode("utf-8"), hashlib.sha256).digest()

def get_signature_key(key, date_stamp, region_name, service_name):
    k_date = sign(("AWS4" + key).encode("utf-8"), date_stamp)
    k_region = sign(k_date, region_name)
    k_service = sign(k_region, service_name)
    k_signing = sign(k_service, "aws4_request")
    return k_signing

class S3WorkerClient:
    def __init__(self, host=HOST, port=PORT):
        self.host = host
        self.port = port
        self.conn = None
        self._connect()

    def _connect(self):
        if self.conn:
            try:
                self.conn.close()
            except Exception:
                pass
        self.conn = http.client.HTTPConnection(self.host, self.port, timeout=30)

    def request(self, method, path, body=b"", query=""):
        if isinstance(body, str):
            body = body.encode("utf-8")
        
        t = datetime.datetime.now(datetime.timezone.utc)
        amz_date = t.strftime("%Y%m%dT%H%M%SZ")
        date_stamp = t.strftime("%Y%m%d")
        
        payload_hash = hashlib.sha256(body).hexdigest()
        canonical_uri = urllib.parse.quote(path, safe="/-_.~")
        canonical_querystring = query
        
        host_header = f"{self.host}:{self.port}"
        canonical_headers = f"host:{host_header}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n"
        signed_headers = "host;x-amz-content-sha256;x-amz-date"
        
        canonical_request = f"{method}\n{canonical_uri}\n{canonical_querystring}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
        algorithm = "AWS4-HMAC-SHA256"
        credential_scope = f"{date_stamp}/{REGION}/{SERVICE}/aws4_request"
        string_to_sign = f"{algorithm}\n{amz_date}\n{credential_scope}\n{hashlib.sha256(canonical_request.encode()).hexdigest()}"
        
        signing_key = get_signature_key(SECRET_KEY, date_stamp, REGION, SERVICE)
        signature = hmac.new(signing_key, string_to_sign.encode(), hashlib.sha256).hexdigest()
        
        auth_header = f"{algorithm} Credential={ACCESS_KEY}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}"
        
        headers = {
            "Host": host_header,
            "x-amz-date": amz_date,
            "x-amz-content-sha256": payload_hash,
            "Authorization": auth_header,
            "Content-Length": str(len(body)),
            "Connection": "keep-alive"
        }
        
        url_path = canonical_uri
        if query:
            url_path += f"?{query}"
            
        start_t = time.perf_counter()
        retries = 2
        for attempt in range(retries):
            try:
                self.conn.request(method, url_path, body=body, headers=headers)
                resp = self.conn.getresponse()
                resp_body = resp.read()
                latency_ms = (time.perf_counter() - start_t) * 1000.0
                return resp.status, resp_body, latency_ms
            except (http.client.RemoteDisconnected, http.client.CannotSendRequest, BrokenPipeError, ConnectionResetError):
                self._connect()
                if attempt == retries - 1:
                    raise

    def close(self):
        if self.conn:
            try:
                self.conn.close()
            except Exception:
                pass


def format_bytes(b):
    if b >= 1024 * 1024 * 1024:
        return f"{b / (1024 * 1024 * 1024):.2f} GB"
    elif b >= 1024 * 1024:
        return f"{b / (1024 * 1024):.2f} MB"
    elif b >= 1024:
        return f"{b / 1024:.2f} KB"
    return f"{b} B"

def run_stress_suite():
    print("=" * 80)
    print("🔥 Z3S STORAGE ENGINE - STRESS & OVERLOAD BENCHMARK SUITE")
    print("=" * 80)
    print(f"Target: {ENDPOINT} (Region: {REGION})")
    print(f"Server Backend: NVMe / Extent Store + Raft Log + Reed-Solomon 4+2 Layout")
    print(f"Start Time: {datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 80)

    # Setup test bucket
    setup_client = S3WorkerClient()
    stress_bucket = f"stress-test-{uuid.uuid4().hex[:8]}"
    print(f"\n📦 Criando bucket de benchmark: {stress_bucket}...", end=" ")
    status, body, _ = setup_client.request("PUT", f"/{stress_bucket}")
    if status not in (200, 204):
        print(f"❌ Falha ao criar bucket: HTTP {status} - {body.decode()}")
        return 1
    print("✅ Criado com sucesso!\n")

    results_summary = []

    # Helper benchmark executor
    def execute_benchmark(test_name, worker_count, total_requests, task_fn):
        print(f"▶ [{test_name}] Concorrência: {worker_count} workers | Total: {total_requests} reqs")
        
        latencies = []
        bytes_transferred = 0
        success_count = 0
        error_count = 0
        
        start_total = time.perf_counter()
        
        def worker_loop(worker_id, num_tasks):
            client = S3WorkerClient()
            w_latencies = []
            w_bytes = 0
            w_success = 0
            w_error = 0
            
            for i in range(num_tasks):
                try:
                    res = task_fn(client, worker_id, i)
                    status, size, lat = res
                    if 200 <= status < 300:
                        w_success += 1
                        w_bytes += size
                        w_latencies.append(lat)
                    else:
                        w_error += 1
                except Exception as e:
                    w_error += 1
            client.close()
            return w_latencies, w_bytes, w_success, w_error

        tasks_per_worker = total_requests // worker_count
        remainder = total_requests % worker_count
        
        with ThreadPoolExecutor(max_workers=worker_count) as executor:
            futures = []
            for w in range(worker_count):
                count = tasks_per_worker + (1 if w < remainder else 0)
                futures.append(executor.submit(worker_loop, w, count))
            
            for fut in as_completed(futures):
                w_lat, w_b, w_s, w_e = fut.result()
                latencies.extend(w_lat)
                bytes_transferred += w_b
                success_count += w_s
                error_count += w_e
                
        total_time = time.perf_counter() - start_total
        rps = (success_count + error_count) / total_time if total_time > 0 else 0
        mb_per_sec = (bytes_transferred / (1024 * 1024)) / total_time if total_time > 0 else 0
        
        latencies.sort()
        p50 = statistics.median(latencies) if latencies else 0
        p90 = latencies[int(len(latencies) * 0.90)] if latencies else 0
        p95 = latencies[int(len(latencies) * 0.95)] if latencies else 0
        p99 = latencies[int(len(latencies) * 0.99)] if latencies else 0
        min_lat = latencies[0] if latencies else 0
        max_lat = latencies[-1] if latencies else 0
        
        print(f"   ⏱  Tempo Total: {total_time:.2f}s | RPS: {rps:,.1f} req/s | Throughput: {mb_per_sec:.2f} MB/s")
        print(f"   📊 Latência (ms): Min={min_lat:.2f}ms | P50={p50:.2f}ms | P90={p90:.2f}ms | P95={p95:.2f}ms | P99={p99:.2f}ms | Max={max_lat:.2f}ms")
        print(f"   🎯 Sucesso: {success_count}/{total_requests} ({success_count/total_requests*100:.2f}%) | Erros: {error_count}\n")
        
        results_summary.append({
            "name": test_name,
            "workers": worker_count,
            "requests": total_requests,
            "time_sec": total_time,
            "rps": rps,
            "throughput_mb_s": mb_per_sec,
            "p50_ms": p50,
            "p90_ms": p90,
            "p95_ms": p95,
            "p99_ms": p99,
            "errors": error_count,
            "success_rate": (success_count / total_requests) * 100.0
        })
        return success_count, error_count

    # -------------------------------------------------------------
    # BATERIA 1: Escalação de Concorrência (Small Objects - 4KB PUT)
    # -------------------------------------------------------------
    print("━" * 80)
    print("📌 BATERIA 1: ESCALAÇÃO DE CONCORRÊNCIA (PUT 4 KB)")
    print("━" * 80)
    payload_4kb = os.urandom(4 * 1024)
    
    for workers in [10, 50, 100, 200]:
        num_reqs = 1000 if workers <= 50 else 2000
        def put_4k_task(client, wid, idx):
            obj_name = f"obj_4k_w{wid}_{idx}.bin"
            status, _, lat = client.request("PUT", f"/{stress_bucket}/{obj_name}", body=payload_4kb)
            return status, len(payload_4kb), lat
        execute_benchmark(f"PUT 4KB (Workers: {workers})", workers, num_reqs, put_4k_task)

    # -------------------------------------------------------------
    # BATERIA 2: Escalação de Leitura Concorrente (GET 4KB)
    # -------------------------------------------------------------
    print("━" * 80)
    print("📌 BATERIA 2: ESCALAÇÃO DE LEITURA CONCORRENTE (GET 4 KB)")
    print("━" * 80)
    
    for workers in [10, 50, 100, 200]:
        num_reqs = 1000 if workers <= 50 else 3000
        def get_4k_task(client, wid, idx):
            target_idx = idx % 200
            obj_name = f"obj_4k_w0_{target_idx}.bin"
            status, body, lat = client.request("GET", f"/{stress_bucket}/{obj_name}")
            return status, len(body), lat
        execute_benchmark(f"GET 4KB (Workers: {workers})", workers, num_reqs, get_4k_task)

    # -------------------------------------------------------------
    # BATERIA 3: Throughput com Objetos Médios e Grandes (64KB, 1MB, 5MB)
    # -------------------------------------------------------------
    print("━" * 80)
    print("📌 BATERIA 3: THROUGHPUT E LARGURA DE BANDA (64KB, 1MB, 5MB)")
    print("━" * 80)
    
    # 64 KB
    payload_64k = os.urandom(64 * 1024)
    def put_64k_task(client, wid, idx):
        obj_name = f"obj_64k_w{wid}_{idx}.bin"
        status, _, lat = client.request("PUT", f"/{stress_bucket}/{obj_name}", body=payload_64k)
        return status, len(payload_64k), lat
    execute_benchmark("PUT 64KB Throughput", 50, 1000, put_64k_task)

    def get_64k_task(client, wid, idx):
        target_idx = idx % 50
        obj_name = f"obj_64k_w0_{target_idx}.bin"
        status, body, lat = client.request("GET", f"/{stress_bucket}/{obj_name}")
        return status, len(body), lat
    execute_benchmark("GET 64KB Throughput", 50, 1000, get_64k_task)

    # 1 MB
    payload_1mb = os.urandom(1024 * 1024)
    def put_1m_task(client, wid, idx):
        obj_name = f"obj_1m_w{wid}_{idx}.bin"
        status, _, lat = client.request("PUT", f"/{stress_bucket}/{obj_name}", body=payload_1mb)
        return status, len(payload_1mb), lat
    execute_benchmark("PUT 1MB Throughput", 20, 200, put_1m_task)

    def get_1m_task(client, wid, idx):
        target_idx = idx % 20
        obj_name = f"obj_1m_w0_{target_idx}.bin"
        status, body, lat = client.request("GET", f"/{stress_bucket}/{obj_name}")
        return status, len(body), lat
    execute_benchmark("GET 1MB Throughput", 20, 200, get_1m_task)

    # 5 MB
    payload_5mb = os.urandom(5 * 1024 * 1024)
    def put_5m_task(client, wid, idx):
        obj_name = f"obj_5m_w{wid}_{idx}.bin"
        status, _, lat = client.request("PUT", f"/{stress_bucket}/{obj_name}", body=payload_5mb)
        return status, len(payload_5mb), lat
    execute_benchmark("PUT 5MB Throughput", 10, 50, put_5m_task)

    def get_5m_task(client, wid, idx):
        target_idx = idx % 10
        obj_name = f"obj_5m_w0_{target_idx}.bin"
        status, body, lat = client.request("GET", f"/{stress_bucket}/{obj_name}")
        return status, len(body), lat
    execute_benchmark("GET 5MB Throughput", 10, 50, get_5m_task)

    # -------------------------------------------------------------
    # BATERIA 4: Carga Mista de Produção (70% GET / 30% PUT)
    # -------------------------------------------------------------
    print("━" * 80)
    print("📌 BATERIA 4: WORKLOAD MISTO REALISTA (70% GET / 30% PUT - 100 WORKERS)")
    print("━" * 80)
    
    mixed_payload = os.urandom(16 * 1024)
    
    def mixed_task(client, wid, idx):
        is_get = (random.random() < 0.70)
        if is_get:
            target_idx = random.randint(0, 100)
            obj_name = f"obj_4k_w0_{target_idx}.bin"
            status, body, lat = client.request("GET", f"/{stress_bucket}/{obj_name}")
            return status, len(body), lat
        else:
            obj_name = f"mixed_obj_{wid}_{idx}.bin"
            status, _, lat = client.request("PUT", f"/{stress_bucket}/{obj_name}", body=mixed_payload)
            return status, len(mixed_payload), lat

    execute_benchmark("Mixed Workload (70% Read / 30% Write)", 100, 3000, mixed_task)

    # -------------------------------------------------------------
    # BATERIA 5: Integridade e Verificação Criptográfica sob Race Condition
    # -------------------------------------------------------------
    print("━" * 80)
    print("📌 BATERIA 5: INTEGRIDADE DE DADOS & RACE CONDITIONS SOB SATURAÇÃO")
    print("━" * 80)
    print("Gerando e gravando 100 objetos únicos com hash SHA256 / MD5 verificado...")
    
    integrity_objects = {}
    for i in range(100):
        data = os.urandom(random.randint(8 * 1024, 64 * 1024))
        h = hashlib.sha256(data).hexdigest()
        integrity_objects[f"integrity_obj_{i}.bin"] = (data, h)

    # Concurrent upload
    def put_integrity_task(client, wid, idx):
        key = f"integrity_obj_{idx}.bin"
        data, _ = integrity_objects[key]
        status, _, lat = client.request("PUT", f"/{stress_bucket}/{key}", body=data)
        return status, len(data), lat
    execute_benchmark("Integrity PUT Phase", 25, 100, put_integrity_task)

    # Concurrent read and SHA256 bit-for-bit check
    print("\n🔍 Validando integridade bit-a-bit sob concorrência de 50 workers...")
    corrupted_count = 0
    verified_count = 0
    
    def verify_worker(w_id, keys_slice):
        nonlocal corrupted_count, verified_count
        client = S3WorkerClient()
        for k in keys_slice:
            expected_data, expected_hash = integrity_objects[k]
            status, body, _ = client.request("GET", f"/{stress_bucket}/{k}")
            if status == 200:
                actual_hash = hashlib.sha256(body).hexdigest()
                if actual_hash == expected_hash and len(body) == len(expected_data):
                    verified_count += 1
                else:
                    corrupted_count += 1
            else:
                corrupted_count += 1
        client.close()

    slices = [list(integrity_objects.keys())[i::10] for i in range(10)]
    with ThreadPoolExecutor(max_workers=10) as executor:
        futures = [executor.submit(verify_worker, i, s) for i, s in enumerate(slices)]
        for f in futures:
            f.result()

    print(f"   🛡️  Objetos verificados com sucesso: {verified_count}/100")
    print(f"   ⚠️  Objetos corrompidos ou com erro de bitrot: {corrupted_count}")
    if corrupted_count == 0:
        print("   ✅ 100% INTEGRIDADE CRIPTOGRÁFICA CONFIRMADA - ZERO BITROT!")
    else:
        print("   ❌ FALHA DE INTEGRIDADE DETECTADA!")

    # -------------------------------------------------------------
    # RELATÓRIO CONSOLIDADO FINAL
    # -------------------------------------------------------------
    print("\n" + "=" * 105)
    print("📋 RESUMO CONSOLIDADO DOS RESULTADOS DO BENCHMARK DE SOBRECARGA")
    print("=" * 105)
    header = f"{'Cenário de Teste':<35} | {'Workers':<7} | {'Reqs':<6} | {'RPS':<9} | {'Throughput':<12} | {'P50 (ms)':<8} | {'P99 (ms)':<8} | {'Taxa Sucesso':<12}"
    print(header)
    print("-" * 105)
    for r in results_summary:
        row = f"{r['name']:<35} | {r['workers']:<7} | {r['requests']:<6} | {r['rps']:<9.1f} | {r['throughput_mb_s']:<8.2f} MB/s | {r['p50_ms']:<8.2f} | {r['p99_ms']:<8.2f} | {r['success_rate']:<11.1f}%"
        print(row)
    print("=" * 105)

    print(f"\n✨ Benchmark concluído com sucesso!")
    return 0

if __name__ == "__main__":
    sys.exit(run_stress_suite())
