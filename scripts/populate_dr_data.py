#!/usr/bin/env python3
"""
Populate control test data on Primary Node (172.16.0.100) for DR validation.
Records exact SHA-256 hashes of all objects in dr_manifest.json.
"""

import os
import sys
import json
import hashlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import stress_benchmark as sb

PRIMARY_HOST = "172.16.0.100"
PORT = 9000

def generate_test_dataset():
    client = sb.S3WorkerClient(host=PRIMARY_HOST, port=PORT)
    
    buckets = [
        "dr-critical-database",
        "dr-media-vault",
        "dr-financial-records"
    ]
    
    # 1. Create buckets
    print("📦 Criando buckets no Nó Primário (172.16.0.100)...")
    for b in buckets:
        status, body, _ = client.request("PUT", f"/{b}")
        print(f"   Bucket '{b}': HTTP {status}")
        assert status in (200, 204), f"Falha ao criar bucket {b}: {body.decode()}"

    # 2. Generate and upload diverse objects
    manifest = {}
    
    test_files = [
        # (bucket, key, size_bytes, description)
        ("dr-critical-database", "db_backup_full.sql", 512 * 1024, "Database SQL dump"),
        ("dr-critical-database", "config.json", 16 * 1024, "System config JSON"),
        ("dr-critical-database", "schema.sql", 64 * 1024, "Schema definition"),
        ("dr-media-vault", "video_asset_1mb.mp4", 1024 * 1024, "1MB Video media chunk"),
        ("dr-media-vault", "image_sample_5mb.raw", 5 * 1024 * 1024, "5MB Raw image payload"),
        ("dr-media-vault", "documents.pdf", 128 * 1024, "PDF Documentation"),
        ("dr-financial-records", "transactions_q1.csv", 256 * 1024, "Financial transaction CSV"),
        ("dr-financial-records", "audit_ledger.bin", 800 * 1024, "Cryptographic audit ledger"),
        ("dr-financial-records", "invoices_2026.xml", 32 * 1024, "Invoicing XML metadata")
    ]
    
    print("\n📤 Enviando objetos de controle e calculando hashes SHA-256...")
    for bucket, key, size, desc in test_files:
        data = os.urandom(size)
        sha256_hash = hashlib.sha256(data).hexdigest()
        
        status, body, lat = client.request("PUT", f"/{bucket}/{key}", body=data)
        print(f"   [{bucket}] {key} ({size / 1024:.1f} KB) - HTTP {status} (lat: {lat:.1f}ms) | SHA256: {sha256_hash[:16]}...")
        assert status in (200, 204), f"Falha ao enviar {key}: {body.decode()}"
        
        manifest[f"{bucket}/{key}"] = {
            "bucket": bucket,
            "key": key,
            "size": size,
            "sha256": sha256_hash,
            "description": desc
        }
        
    client.close()
    
    with open("dr_manifest.json", "w") as f:
        json.dump(manifest, f, indent=2)
        
    print(f"\n✅ Manifesto de controle salvo em dr_manifest.json ({len(manifest)} objetos cadastrados)!")

if __name__ == "__main__":
    generate_test_dataset()
