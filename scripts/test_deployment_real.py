#!/usr/bin/env python3
"""
Test script for real dual-node Z3S deployment:
- Node 1: 192.168.122.11 (Active)
- Node 2: 192.168.122.12 (Standby / Disaster Recovery)
"""

import sys
import os
import time
import json
import hashlib
import subprocess

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import stress_benchmark as sb

NODE1_IP = "192.168.122.11"
NODE2_IP = "192.168.122.12"
PORT = 9000

def main():
    print("=" * 80)
    print("🚀 TESTE REAL DE IMPLANTAÇÃO E OPERAÇÃO Z3S CLUSTER (DUAL-NODE)")
    print("=" * 80)
    
    # 1. Conexão ao Nó 1
    print(f"\n[1/6] 🌐 Conectando à API S3 do Nó 1 ({NODE1_IP}:{PORT})...")
    client1 = sb.S3WorkerClient(host=NODE1_IP, port=PORT)
    status, body, lat = client1.request("GET", "/")
    assert status == 200, f"Falha na listagem de buckets no Nó 1: {status}"
    print(f"   ✅ Conexão estabelecida com sucesso (latência: {lat:.2f} ms).")

    # 2. Criação de Bucket
    bucket = "producao-dados-criticos"
    print(f"\n[2/6] 📦 Criando bucket '{bucket}' no Nó 1...")
    status, body, lat = client1.request("PUT", f"/{bucket}")
    assert status in (200, 204), f"Erro criando bucket: HTTP {status} - {body.decode()}"
    print(f"   ✅ Bucket criado com sucesso (HTTP {status}, latência: {lat:.2f} ms).")

    # 3. Upload de Objetos Heterogêneos
    print("\n[3/6] 📤 Realizando uploads de controle no Nó 1 com cálculo de hash...")
    test_payloads = [
        ("manifesto-sistema.json", json.dumps({"cluster": "z3s-prod", "nodes": [NODE1_IP, NODE2_IP], "status": "active"}).encode()),
        ("relatorio-financeiro.txt", ("Registro de transações seguras Z3S\n" * 100).encode()),
        ("dataset-binario-1mb.bin", os.urandom(1024 * 1024)),
        ("dados-alta-concorrencia-4mb.raw", os.urandom(4 * 1024 * 1024))
    ]

    manifest = {}
    for key, data in test_payloads:
        sha256 = hashlib.sha256(data).hexdigest()
        status, body, lat = client1.request("PUT", f"/{bucket}/{key}", body=data)
        assert status in (200, 204), f"Falha no upload de {key}: HTTP {status}"
        manifest[key] = {"size": len(data), "sha256": sha256}
        print(f"   ✅ [{key}] Tamanho: {len(data)/1024:.1f} KB | SHA256: {sha256[:16]}... | Lat: {lat:.2f} ms")

    # 4. Verificação de Leitura Bit-a-Bit no Nó 1
    print("\n[4/6] 📥 Validando integridade bit-a-bit dos objetos gravados no Nó 1...")
    for key, info in manifest.items():
        status, body, lat = client1.request("GET", f"/{bucket}/{key}")
        assert status == 200, f"Falha no download de {key}: HTTP {status}"
        actual_hash = hashlib.sha256(body).hexdigest()
        assert actual_hash == info["sha256"], f"Corrupção detectada em {key}!"
        assert len(body) == info["size"], f"Tamanho divergente em {key}!"
        print(f"   ✅ [{key}] Download íntegro validado 100% bit-a-bit (latência: {lat:.2f} ms).")

    client1.close()

    # 5. Sincronização DR (Disaster Recovery) para Nó 2
    print(f"\n[5/6] 🔄 Disparando sincronização DR entre Nó 1 e Nó 2 via rsync...")
    res = subprocess.run([
        "ssh", f"root@{NODE1_IP}",
        f'rsync -avz --delete -e "ssh -o StrictHostKeyChecking=no" /mnt/dados/z3s-data/ root@{NODE2_IP}:/mnt/dados/z3s-data/'
    ], capture_output=True, text=True)
    assert res.returncode == 0, f"Falha na sincronização DR: {res.stderr}"
    print("   ✅ Replicação DR concluída com sucesso:")
    for line in res.stdout.strip().split("\n"):
        if line:
            print(f"      {line}")

    # 6. Simulação de Failover no Nó 2: Iniciar Z3S e Ler Dados Replicados
    print(f"\n[6/6] 🛡️ Validando capacidade de Failover no Nó 2 ({NODE2_IP}:{PORT})...")
    
    # Iniciar Z3S no Nó 2
    print(f"   Iniciando serviço Z3S no Nó 2...")
    res = subprocess.run([
        "ssh", f"root@{NODE2_IP}",
        "rc-service z3s restart && sleep 1 && rc-service z3s status"
    ], capture_output=True, text=True)
    print(f"   Status Nó 2: {res.stdout.strip()}")

    client2 = sb.S3WorkerClient(host=NODE2_IP, port=PORT)
    status, body, lat = client2.request("GET", "/")
    assert status == 200, f"Falha na conexão com Nó 2: HTTP {status}"
    
    # Validar que todos os objetos do manifesto estão íntegros no Nó 2
    print(f"   Verificando integridade dos dados no Nó 2 após failover...")
    for key, info in manifest.items():
        status, body, lat = client2.request("GET", f"/{bucket}/{key}")
        assert status == 200, f"Nó 2 não encontrou {key}: HTTP {status}"
        actual_hash = hashlib.sha256(body).hexdigest()
        assert actual_hash == info["sha256"], f"Corrupção no Nó 2 para {key}!"
        print(f"   ✅ [Nó 2 -> {key}] Integridade 100% confirmada! SHA256 confere com o original.")

    client2.close()

    print("\n" + "=" * 80)
    print("🎉 SUCESSO TOTAL! IMPLANTAÇÃO EM PRODUÇÃO VALIDADA EM AMBOS OS SERVIDORES!")
    print("=" * 80)

if __name__ == "__main__":
    main()
