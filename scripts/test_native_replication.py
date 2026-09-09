#!/usr/bin/env python3
"""
Test Native Peer-to-Peer Replication between Z3S Node 1 and Node 2.
No cron. No rsync. 100% Application Layer S3 Replication.
"""

import os
import sys
import time
import hashlib
import json

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import stress_benchmark as sb

NODE1_HOST = "192.168.122.11"
NODE2_HOST = "192.168.122.12"
PORT = 9000

def main():
    print("=" * 80)
    print("🚀 TESTE DE REPLICAÇÃO NATIVA P2P NO Z3S (CAMADA DE APLICAÇÃO)")
    print("=" * 80)

    client1 = sb.S3WorkerClient(host=NODE1_HOST, port=PORT)
    client2 = sb.S3WorkerClient(host=NODE2_HOST, port=PORT)

    # 1. Criação de Bucket no Nó 1
    bucket = "bucket-replicacao-nativa"
    print(f"\n[1/4] 📦 Verificando/Criando bucket '{bucket}' no Nó 1 ({NODE1_HOST})...")
    status, body, lat = client1.request("PUT", f"/{bucket}")
    assert status in (200, 204, 409), f"Falha ao criar bucket no Nó 1: {status} - {body.decode()}"
    print(f"   ✅ Bucket pronto no Nó 1 (HTTP {status}, {lat:.1f}ms).")

    # Espera 500ms para a replicação assíncrona viajar pela rede
    time.sleep(0.5)

    # 2. Verificação do Bucket no Nó 2 sem nenhum restart ou rsync
    print(f"\n[2/4] 🔍 Consultando Nó 2 ({NODE2_HOST}) para verificar replicação do bucket...")
    status, body, lat = client2.request("GET", "/")
    assert status == 200, f"Falha ao listar buckets no Nó 2: {status}"
    body_str = body.decode("utf-8", errors="ignore")
    assert bucket in body_str, f"Bucket {bucket} não foi replicado para o Nó 2!"
    print(f"   ✅ Bucket '{bucket}' encontrado no Nó 2 em tempo real via API! (HTTP {status})")

    # 3. Upload de Objeto com Dados Originais no Nó 1
    key = "documento-critico-nativa.bin"
    payload = os.urandom(512 * 1024) # 512 KB
    expected_sha256 = hashlib.sha256(payload).hexdigest()
    print(f"\n[3/4] 📤 Enviando objeto de 512 KB '{key}' para o Nó 1...")
    print(f"   SHA-256 Original: {expected_sha256}")
    status, body, lat = client1.request("PUT", f"/{bucket}/{key}", body=payload)
    assert status in (200, 204), f"Falha no upload para Nó 1: {status}"
    print(f"   ✅ Upload concluído no Nó 1 (HTTP {status}, {lat:.1f}ms).")

    # Espera 500ms para a replicação assíncrona
    time.sleep(0.5)

    # 4. Download Imediato a partir do Nó 2
    print(f"\n[4/4] 📥 Baixando objeto diretamente do Nó 2 ({NODE2_HOST}) e validando integridade...")
    status, body, lat = client2.request("GET", f"/{bucket}/{key}")
    assert status == 200, f"Nó 2 retornou HTTP {status} (objeto não encontrado ou erro): {body.decode()}"
    actual_sha256 = hashlib.sha256(body).hexdigest()
    print(f"   SHA-256 Recebido do Nó 2: {actual_sha256}")
    assert actual_sha256 == expected_sha256, "Corrupção detectada! O hash no Nó 2 não confere!"
    assert len(body) == len(payload), "Tamanho divergente do payload recebido!"
    print(f"   ✅ SUCESSO ABSOLUTO: Integridade 100% bit-a-bit confirmada no Nó 2! (latência: {lat:.1f}ms)")

    client1.close()
    client2.close()

    print("\n" + "=" * 80)
    print("🎉 REPLICAÇÃO NATIVA DA APLICAÇÃO Z3S FUNCIONANDO PERFEITAMENTE EM TEMPO REAL!")
    print("=" * 80)

if __name__ == "__main__":
    main()
