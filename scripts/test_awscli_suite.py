#!/usr/bin/env python3
import subprocess
import json
import hashlib
import uuid
import os
import sys
import time

ENDPOINT = "http://127.0.0.1:9000"
AWS_CMD = ["aws", "--endpoint-url", ENDPOINT]

passed_tests = 0
failed_tests = 0
total_tests = 0

def run_aws(args, check=True, return_json=False):
    cmd = AWS_CMD + args
    res = subprocess.run(cmd, capture_output=True, text=True)
    if check and res.returncode != 0:
        raise RuntimeError(f"Command failed: {' '.join(cmd)}\nStderr: {res.stderr}\nStdout: {res.stdout}")
    if return_json:
        try:
            return json.loads(res.stdout)
        except json.JSONDecodeError:
            raise RuntimeError(f"Expected JSON output from: {' '.join(cmd)}\nReceived: {res.stdout}")
    return res

def test_case(suite_name, test_name):
    def decorator(func):
        def wrapper(*args, **kwargs):
            global passed_tests, failed_tests, total_tests
            total_tests += 1
            print(f"[{suite_name}] 🔄 Executando: {test_name}...", end=" ", flush=True)
            try:
                func(*args, **kwargs)
                passed_tests += 1
                print("✅ PASSOU")
            except Exception as e:
                failed_tests += 1
                print(f"❌ FALHOU!\n   Erro: {e}")
        return wrapper
    return decorator

# ==========================================
# BATERIA 1: Buckets
# ==========================================
@test_case("Bateria 1: Buckets", "1.1 Criar bucket (Make Bucket)")
def test_1_1(bucket):
    run_aws(["s3", "mb", f"s3://{bucket}"])

@test_case("Bateria 1: Buckets", "1.2 Listar buckets e verificar presenca")
def test_1_2(bucket):
    res = run_aws(["s3", "ls"])
    assert bucket in res.stdout, f"Bucket {bucket} não encontrado em s3 ls"

@test_case("Bateria 1: Buckets", "1.3 Head Bucket")
def test_1_3(bucket):
    run_aws(["s3api", "head-bucket", "--bucket", bucket])

@test_case("Bateria 1: Buckets", "1.4 Remover bucket vazio")
def test_1_4(bucket_to_delete):
    run_aws(["s3", "mb", f"s3://{bucket_to_delete}"])
    run_aws(["s3", "rb", f"s3://{bucket_to_delete}"])
    res = run_aws(["s3", "ls"])
    assert bucket_to_delete not in res.stdout, f"Bucket {bucket_to_delete} ainda presente após rb"

# ==========================================
# BATERIA 2: Objetos Básicos (CRUD)
# ==========================================
@test_case("Bateria 2: Objetos", "2.1 Upload e Download de arquivo de texto com comparacao de hash")
def test_2_1(bucket):
    local_src = "/tmp/z3s_test_text.txt"
    local_dst = "/tmp/z3s_test_text_down.txt"
    content = f"Z3S Storage Teste de Integridade - Timestamp: {time.time()} - UUID: {uuid.uuid4()}"
    with open(local_src, "w") as f:
        f.write(content)

    run_aws(["s3", "cp", local_src, f"s3://{bucket}/texto.txt"])
    run_aws(["s3", "cp", f"s3://{bucket}/texto.txt", local_dst])

    with open(local_dst, "r") as f:
        down_content = f.read()

    assert content == down_content, "Conteúdo baixado difere do original"

@test_case("Bateria 2: Objetos", "2.2 Upload de arquivo vazio (0 bytes)")
def test_2_2(bucket):
    local_empty = "/tmp/z3s_empty.dat"
    open(local_empty, "w").close()
    run_aws(["s3", "cp", local_empty, f"s3://{bucket}/empty.dat"])
    head = run_aws(["s3api", "head-object", "--bucket", bucket, "--key", "empty.dat"], return_json=True)
    assert head["ContentLength"] == 0, f"Tamanho do arquivo vazio incorreto: {head['ContentLength']}"

@test_case("Bateria 2: Objetos", "2.3 Head Object e validacao de metadados")
def test_2_3(bucket):
    head = run_aws(["s3api", "head-object", "--bucket", bucket, "--key", "texto.txt"], return_json=True)
    assert "ContentLength" in head and head["ContentLength"] > 0
    assert "ETag" in head

@test_case("Bateria 2: Objetos", "2.4 Copia interna de objeto (CopyObject)")
def test_2_4(bucket):
    run_aws(["s3", "cp", f"s3://{bucket}/texto.txt", f"s3://{bucket}/copia_texto.txt"])
    head = run_aws(["s3api", "head-object", "--bucket", bucket, "--key", "copia_texto.txt"], return_json=True)
    assert head["ContentLength"] > 0

@test_case("Bateria 2: Objetos", "2.5 Exclusao de objeto (DeleteObject)")
def test_2_5(bucket):
    run_aws(["s3", "rm", f"s3://{bucket}/copia_texto.txt"])
    res = run_aws(["s3api", "head-object", "--bucket", bucket, "--key", "copia_texto.txt"], check=False)
    assert res.returncode != 0, "Objeto deletado ainda foi encontrado"

# ==========================================
# BATERIA 3: Pastas Virtuais & ListObjectsV2
# ==========================================
@test_case("Bateria 3: Pastas & List", "3.1 Upload de arvore hierarquica de pastas")
def test_3_1(bucket):
    tmp_file = "/tmp/z3s_dummy.txt"
    with open(tmp_file, "w") as f:
        f.write("dummy")
    run_aws(["s3", "cp", tmp_file, f"s3://{bucket}/pasta_a/sub1/arq1.txt"])
    run_aws(["s3", "cp", tmp_file, f"s3://{bucket}/pasta_a/sub1/arq2.txt"])
    run_aws(["s3", "cp", tmp_file, f"s3://{bucket}/pasta_a/sub2/arq3.txt"])
    run_aws(["s3", "cp", tmp_file, f"s3://{bucket}/pasta_b/arq4.txt"])

@test_case("Bateria 3: Pastas & List", "3.2 ListObjectsV2 com prefixo e delimitador (CommonPrefixes)")
def test_3_2(bucket):
    res = run_aws(["s3api", "list-objects-v2", "--bucket", bucket, "--prefix", "pasta_a/", "--delimiter", "/"], return_json=True)
    prefixes = [p["Prefix"] for p in res.get("CommonPrefixes", [])]
    assert "pasta_a/sub1/" in prefixes, f"Subpasta 1 nao encontrada: {prefixes}"
    assert "pasta_a/sub2/" in prefixes, f"Subpasta 2 nao encontrada: {prefixes}"

@test_case("Bateria 3: Pastas & List", "3.3 ListObjectsV2 com paginacao (max-keys)")
def test_3_3(bucket):
    res = run_aws(["s3api", "list-objects-v2", "--bucket", bucket, "--max-items", "2"], return_json=True)
    assert len(res.get("Contents", [])) <= 2

# ==========================================
# BATERIA 4: Versionamento de Objetos
# ==========================================
@test_case("Bateria 4: Versionamento", "4.1 Habilitar versionamento no bucket")
def test_4_1(bucket):
    run_aws(["s3api", "put-bucket-versioning", "--bucket", bucket, "--versioning-configuration", "Status=Enabled"])
    res = run_aws(["s3api", "get-bucket-versioning", "--bucket", bucket], return_json=True)
    assert res.get("Status") == "Enabled"

@test_case("Bateria 4: Versionamento", "4.2 Sobrescrita sucessiva e geracao de versoes")
def test_4_2(bucket):
    key = "versao_teste.txt"
    f1 = "/tmp/v1.txt"
    f2 = "/tmp/v2.txt"
    with open(f1, "w") as f:
        f.write("Versao 1")
    with open(f2, "w") as f:
        f.write("Versao 2")

    run_aws(["s3", "cp", f1, f"s3://{bucket}/{key}"])
    time.sleep(0.05)
    run_aws(["s3", "cp", f2, f"s3://{bucket}/{key}"])

    versions = run_aws(["s3api", "list-object-versions", "--bucket", bucket, "--prefix", key], return_json=True)
    obj_versions = [v for v in versions.get("Versions", []) if v["Key"] == key]
    assert len(obj_versions) >= 2, f"Esperado ao menos 2 versoes, encontrado {len(obj_versions)}"
    assert obj_versions[0]["IsLatest"] == True

@test_case("Bateria 4: Versionamento", "4.3 Delete Marker e recuperacao de versao historica")
def test_4_3(bucket):
    key = "versao_teste.txt"
    versions = run_aws(["s3api", "list-object-versions", "--bucket", bucket, "--prefix", key], return_json=True)
    obj_versions = [v for v in versions.get("Versions", []) if v["Key"] == key]
    older_version_id = obj_versions[1]["VersionId"]

    # Deleta objeto -> cria Delete Marker
    run_aws(["s3", "rm", f"s3://{bucket}/{key}"])

    # Baixa versao historica especifica
    dest = "/tmp/v_hist.txt"
    run_aws(["s3api", "get-object", "--bucket", bucket, "--key", key, "--version-id", older_version_id, dest])
    with open(dest, "r") as f:
        content = f.read()
    assert content == "Versao 1", f"Conteudo da versao historica incorreto: {content}"

# ==========================================
# BATERIA 5: Multipart Uploads
# ==========================================
@test_case("Bateria 5: Multipart", "5.1 Upload multipart automatico de arquivo grande (20 MB)")
def test_5_1(bucket):
    large_file = "/tmp/z3s_large_20m.bin"
    size = 20 * 1024 * 1024
    with open(large_file, "wb") as f:
        f.write(os.urandom(size))

    orig_hash = hashlib.md5(open(large_file, "rb").read()).hexdigest()

    run_aws(["s3", "cp", large_file, f"s3://{bucket}/large_20m.bin"])

    # Download e verificacao de hash MD5
    down_file = "/tmp/z3s_large_20m_down.bin"
    run_aws(["s3", "cp", f"s3://{bucket}/large_20m.bin", down_file])
    down_hash = hashlib.md5(open(down_file, "rb").read()).hexdigest()

    assert orig_hash == down_hash, "Hash do arquivo baixado difere do arquivo de 20MB original"

# ==========================================
# BATERIA 6: Criptografia SSE & Politicas
# ==========================================
@test_case("Bateria 6: Seguranca", "6.1 Configuracao de Criptografia SSE-S3 padrao no bucket")
def test_6_1(bucket):
    enc_config = json.dumps({
        "Rules": [{"ApplyServerSideEncryptionByDefault": {"SSEAlgorithm": "AES256"}}]
    })
    run_aws(["s3api", "put-bucket-encryption", "--bucket", bucket, "--server-side-encryption-configuration", enc_config])
    res = run_aws(["s3api", "get-bucket-encryption", "--bucket", bucket], return_json=True)
    algo = res["ServerSideEncryptionConfiguration"]["Rules"][0]["ApplyServerSideEncryptionByDefault"]["SSEAlgorithm"]
    assert algo == "AES256", f"Algoritmo SSE incorreto: {algo}"

@test_case("Bateria 6: Seguranca", "6.2 Upload com cabecalho explicito SSE-KMS e decriptografia transparente")
def test_6_2(bucket):
    f_sec = "/tmp/z3s_secret.txt"
    f_sec_down = "/tmp/z3s_secret_down.txt"
    sec_content = "Top Secret KMS Envelope Encrypted Payload!"
    with open(f_sec, "w") as f:
        f.write(sec_content)

    run_aws(["s3", "cp", f_sec, f"s3://{bucket}/secret_kms.txt", "--sse", "aws:kms"])
    head = run_aws(["s3api", "head-object", "--bucket", bucket, "--key", "secret_kms.txt"], return_json=True)
    assert head.get("ServerSideEncryption") == "aws:kms"

    run_aws(["s3", "cp", f"s3://{bucket}/secret_kms.txt", f_sec_down])
    with open(f_sec_down, "r") as f:
        assert f.read() == sec_content, "Conteudo descriptografado incorreto"

# ==========================================
# BATERIA 7: Lifecycle
# ==========================================
@test_case("Bateria 7: Lifecycle", "7.1 Configuracao, Consulta e Remocao de Regras de Ciclo de Vida")
def test_7_1(bucket):
    lc_config = json.dumps({
        "Rules": [
            {
                "ID": "Rule-Logs-30d",
                "Status": "Enabled",
                "Filter": {"Prefix": "logs/"},
                "Expiration": {"Days": 30},
                "Transitions": [{"Days": 10, "StorageClass": "GLACIER"}]
            }
        ]
    })
    run_aws(["s3api", "put-bucket-lifecycle-configuration", "--bucket", bucket, "--lifecycle-configuration", lc_config])
    res = run_aws(["s3api", "get-bucket-lifecycle-configuration", "--bucket", bucket], return_json=True)
    assert len(res["Rules"]) == 1
    assert res["Rules"][0]["ID"] == "Rule-Logs-30d"

    run_aws(["s3api", "delete-bucket-lifecycle", "--bucket", bucket])
    res_del = run_aws(["s3api", "get-bucket-lifecycle-configuration", "--bucket", bucket], check=False)
    assert res_del.returncode != 0

# ==========================================
# BATERIA 8: Byte-Range Requests
# ==========================================
@test_case("Bateria 8: Byte-Range", "8.1 Leitura parcial de intervalo de bytes (Byte-Range)")
def test_8_1(bucket):
    f_range = "/tmp/z3s_range.txt"
    f_range_part = "/tmp/z3s_range_slice.txt"
    alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    with open(f_range, "w") as f:
        f.write(alphabet)

    run_aws(["s3", "cp", f_range, f"s3://{bucket}/range_test.txt"])

    # Solicita bytes 10 a 19 (10 bytes: "KLMNOPQRST")
    run_aws(["s3api", "get-object", "--bucket", bucket, "--key", "range_test.txt", "--range", "bytes=10-19", f_range_part])
    with open(f_range_part, "r") as f:
        slice_content = f.read()

    assert slice_content == "KLMNOPQRST", f"Intervalo de bytes incorreto: {slice_content}"


def main():
    print("================================================================================")
    print("🚀 INICIANDO SUÍTE COMPLETA DE TESTES OFICIAIS AWS CLI CONTRA O Z3S")
    print(f"📍 Endpoint: {ENDPOINT}")
    print("================================================================================")

    main_bucket = f"z3s-test-suite-{uuid.uuid4().hex[:8]}"
    temp_bucket = f"z3s-temp-suite-{uuid.uuid4().hex[:8]}"

    # Bateria 1
    test_1_1(main_bucket)
    test_1_2(main_bucket)
    test_1_3(main_bucket)
    test_1_4(temp_bucket)

    # Bateria 2
    test_2_1(main_bucket)
    test_2_2(main_bucket)
    test_2_3(main_bucket)
    test_2_4(main_bucket)
    test_2_5(main_bucket)

    # Bateria 3
    test_3_1(main_bucket)
    test_3_2(main_bucket)
    test_3_3(main_bucket)

    # Bateria 4
    test_4_1(main_bucket)
    test_4_2(main_bucket)
    test_4_3(main_bucket)

    # Bateria 5
    test_5_1(main_bucket)

    # Bateria 6
    test_6_1(main_bucket)
    test_6_2(main_bucket)

    # Bateria 7
    test_7_1(main_bucket)

    # Bateria 8
    test_8_1(main_bucket)

    print("\n================================================================================")
    print(f"🏁 RESULTADO FINAL: {passed_tests}/{total_tests} TESTES PASSARAM COM SUCESSO!")
    print("================================================================================")

    if failed_tests > 0:
        sys.exit(1)

if __name__ == "__main__":
    main()
