#!/usr/bin/env python3
"""
Test script for User-Driven Replication in Z3S
Tests:
1. Dynamic configuration via API / UI
2. Real-time SigV4 signed replication of buckets, objects, and deletes
3. Disaster Recovery: reading identical content from peer node 2
4. Disabling replication and verifying standalone behavior
"""

import os
import sys
import time
import json
import urllib.request
import urllib.error

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from stress_benchmark import S3WorkerClient

NODE1 = "192.168.122.11"
NODE2 = "192.168.122.12"
PORT = 9000

def post_json(url, payload):
    req = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"},
        method="POST"
    )
    with urllib.request.urlopen(req) as resp:
        return json.loads(resp.read().decode("utf-8"))

def get_json(url):
    req = urllib.request.Request(url, headers={"Accept": "application/json"})
    with urllib.request.urlopen(req) as resp:
        return json.loads(resp.read().decode("utf-8"))

def run_test():
    print("================================================================")
    print("🔍 TESTE DE REPLICAÇÃO DINÂMICA CONFIGURADA PELO USUÁRIO (HA/DR)")
    print("================================================================")

    # 1. Configura a replicação no Node 1 apontando para o Node 2 via API (simulando a ação do usuário no Console Web)
    print("\n1️⃣ Usuário configurando e ativando a replicação no Servidor 1 (192.168.122.11)...")
    config_payload = {
        "enabled": True,
        "peer_endpoint": f"http://{NODE2}:{PORT}",
        "peer_access_key": "Z3SACCESSKEYEXAMPLE",
        "peer_secret_key": "Z3SSECRETKEYEXAMPLE1234567890ABCDEF",
        "peer_region": "us-east-1"
    }
    res_enable = post_json(f"http://{NODE1}:{PORT}/z3s/api/cluster/replication", config_payload)
    print(f"   Status da Replicação: {res_enable['status']} | Peer: {res_enable['peer_endpoint']} | Anti-Loop: {res_enable['anti_loop_header']}")
    assert res_enable["enabled"] is True
    assert res_enable["status"] == "ACTIVE"

    # 2. Cliente S3 oficial autenticado no Node 1 e Node 2
    c1 = S3WorkerClient(host=NODE1, port=PORT)
    c2 = S3WorkerClient(host=NODE2, port=PORT)

    bucket_name = "ha-user-vault"
    print(f"\n2️⃣ Criando bucket '{bucket_name}' no Servidor 1...")
    status, body, _ = c1.request("PUT", f"/{bucket_name}")
    print(f"   Node 1 PUT /{bucket_name}: HTTP {status}")
    assert status in (200, 204)

    time.sleep(0.5)

    print(f"   Verificando replicação imediata do bucket '{bucket_name}' no Servidor 2...")
    status, _, _ = c2.request("HEAD", f"/{bucket_name}")
    print(f"   Node 2 HEAD /{bucket_name}: HTTP {status}")
    assert status == 200, f"Bucket não replicado no Node 2! HTTP {status}"
    print("   ✅ Bucket replicado com sucesso!")

    # 3. Upload de objeto no Node 1
    object_key = "documento-estratégico.iso"
    test_payload = b"CONTEUDO ORIGINAL DO ARQUIVO ISO GRAVADO NO SERVIDOR 1. REPLICADO 100% BIT-A-BIT VIA TOKIO E SIGV4!"
    print(f"\n3️⃣ Enviando objeto '{object_key}' ({len(test_payload)} bytes) para o Servidor 1...")
    status, body, _ = c1.request("PUT", f"/{bucket_name}/{object_key}", body=test_payload)
    print(f"   Node 1 PUT /{bucket_name}/{object_key}: HTTP {status}")
    assert status in (200, 204)

    time.sleep(0.5)

    # 4. Leitura do objeto original diretamente do Node 2 (Disaster Recovery)
    print(f"\n4️⃣ Lendo objeto '{object_key}' diretamente no Servidor 2 (Destino)...")
    status, body_node2, _ = c2.request("GET", f"/{bucket_name}/{object_key}")
    print(f"   Node 2 GET /{bucket_name}/{object_key}: HTTP {status} | Tamanho: {len(body_node2)} bytes")
    assert status == 200, f"Objeto não encontrado no Node 2! HTTP {status}"
    assert body_node2 == test_payload, "Conteúdo do objeto no Node 2 diverge do original enviado ao Node 1!"
    print(f"   ✅ Conteúdo validado com sucesso! Dados 100% idênticos no Node 2:")
    print(f"      \"{body_node2.decode('utf-8')}\"")

    # 5. Exclusão de objeto no Node 1
    print(f"\n5️⃣ Excluindo objeto '{object_key}' no Servidor 1...")
    status, _, _ = c1.request("DELETE", f"/{bucket_name}/{object_key}")
    print(f"   Node 1 DELETE /{bucket_name}/{object_key}: HTTP {status}")
    assert status in (200, 204)

    time.sleep(0.5)

    print(f"   Verificando se o objeto foi excluído também no Servidor 2...")
    status, _, _ = c2.request("GET", f"/{bucket_name}/{object_key}")
    print(f"   Node 2 GET /{bucket_name}/{object_key}: HTTP {status} (Esperado 404 NoSuchKey)")
    assert status == 404, f"Objeto ainda presente no Node 2 após exclusão! HTTP {status}"
    print("   ✅ Exclusão replicada com sucesso!")

    # 6. Usuário desativando a replicação
    print("\n6️⃣ Usuário desativando a replicação no Servidor 1...")
    res_disable = post_json(f"http://{NODE1}:{PORT}/z3s/api/cluster/replication", {"enabled": False})
    print(f"   Status da Replicação: {res_disable['status']} (Enabled: {res_disable['enabled']})")
    assert res_disable["enabled"] is False

    print("\n7️⃣ Enviando arquivo com replicação desligada...")
    status, _, _ = c1.request("PUT", f"/{bucket_name}/standalone.txt", body=b"standalone-only")
    assert status in (200, 204)

    time.sleep(0.5)

    status, _, _ = c2.request("GET", f"/{bucket_name}/standalone.txt")
    print(f"   Node 2 GET /{bucket_name}/standalone.txt: HTTP {status} (Esperado 404 pois a replicação estava desativada)")
    assert status == 404, "Arquivo não deveria ter sido replicado após desativação da replicação!"
    print("   ✅ Servidor 1 operando isolado (Standalone) com sucesso!")

    print("\n================================================================")
    print("🎉 TODOS OS TESTES DE REPLICAÇÃO DO USUÁRIO PASSARAM COM SUCESSO!")
    print("================================================================")

if __name__ == "__main__":
    run_test()
