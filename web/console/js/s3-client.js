/**
 * Z3S S3 REST Client with AWS Signature Version 4 (SigV4) Signing
 * Uses native Web Crypto API (SubtleCrypto) - Zero external dependencies.
 */

class S3Client {
  constructor(endpoint, accessKey, secretKey, region = "us-east-1") {
    this.endpoint = endpoint.replace(/\/$/, "");
    this.accessKey = accessKey;
    this.secretKey = secretKey;
    this.region = region;
    this.service = "s3";
  }

  // --- Web Crypto Helpers ---
  async sha256(data) {
    const encoder = new TextEncoder();
    const buffer = typeof data === "string" ? encoder.encode(data) : data;
    const hash = await crypto.subtle.digest("SHA-256", buffer);
    return Array.from(new Uint8Array(hash))
      .map(b => b.toString(16).padStart(2, "0"))
      .join("");
  }

  async hmacRaw(key, data) {
    const encoder = new TextEncoder();
    const dataBuffer = typeof data === "string" ? encoder.encode(data) : data;
    const cryptoKey = await crypto.subtle.importKey(
      "raw",
      key,
      { name: "HMAC", hash: { name: "SHA-256" } },
      false,
      ["sign"]
    );
    const signature = await crypto.subtle.sign("HMAC", cryptoKey, dataBuffer);
    return new Uint8Array(signature);
  }

  async getSignatureKey(key, dateStamp, regionName, serviceName) {
    const encoder = new TextEncoder();
    const kDate = await this.hmacRaw(encoder.encode("AWS4" + key), dateStamp);
    const kRegion = await this.hmacRaw(kDate, regionName);
    const kService = await this.hmacRaw(kRegion, serviceName);
    const kSigning = await this.hmacRaw(kService, "aws4_request");
    return kSigning;
  }

  // --- SigV4 Header Construction ---
  async signRequest(method, path, queryParams = {}, body = "", customHeaders = {}) {
    const now = new Date();
    const amzDate = now.toISOString().replace(/[:-]|\.\d{3}/g, "");
    const dateStamp = amzDate.substring(0, 8);

    const url = new URL(this.endpoint + path);
    const host = url.host;

    let payloadHash;
    if (body instanceof Uint8Array || body instanceof ArrayBuffer) {
      payloadHash = await this.sha256(body);
    } else if (typeof body === "string" && body.length > 0) {
      payloadHash = await this.sha256(body);
    } else {
      payloadHash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    }

    const headers = {
      host: host,
      "x-amz-date": amzDate,
      "x-amz-content-sha256": payloadHash,
      ...customHeaders
    };

    // Sort query string
    const sortedKeys = Object.keys(queryParams).sort();
    const canonicalQuery = sortedKeys
      .map(k => `${encodeURIComponent(k)}=${encodeURIComponent(queryParams[k] || "")}`)
      .join("&");

    // Canonical headers
    const sortedHeaderKeys = Object.keys(headers).map(k => k.toLowerCase()).sort();
    const canonicalHeaders = sortedHeaderKeys
      .map(k => `${k}:${headers[k].trim()}\n`)
      .join("");
    const signedHeaders = sortedHeaderKeys.join(";");

    const canonicalUri = encodeURI(path);
    const canonicalRequest = `${method}\n${canonicalUri}\n${canonicalQuery}\n${canonicalHeaders}\n${signedHeaders}\n${payloadHash}`;

    const algorithm = "AWS4-HMAC-SHA256";
    const credentialScope = `${dateStamp}/${this.region}/${this.service}/aws4_request`;
    const stringToSign = `${algorithm}\n${amzDate}\n${credentialScope}\n${await this.sha256(canonicalRequest)}`;

    const signingKey = await this.getSignatureKey(this.secretKey, dateStamp, this.region, this.service);
    const signatureBytes = await this.hmacRaw(signingKey, stringToSign);
    const signature = Array.from(signatureBytes).map(b => b.toString(16).padStart(2, "0")).join("");

    const authorization = `${algorithm} Credential=${this.accessKey}/${credentialScope}, SignedHeaders=${signedHeaders}, Signature=${signature}`;

    const finalHeaders = {
      ...headers,
      Authorization: authorization
    };

    let targetUrl = this.endpoint + path;
    if (canonicalQuery) {
      targetUrl += `?${canonicalQuery}`;
    }

    return { url: targetUrl, headers: finalHeaders };
  }

  // --- High Level S3 Operations ---

  async listBuckets() {
    const { url, headers } = await this.signRequest("GET", "/");
    const res = await fetch(url, { method: "GET", headers });
    if (!res.ok) {
      throw new Error(`Erro ao listar buckets: HTTP ${res.status}`);
    }
    const text = await res.text();
    const parser = new DOMParser();
    const xml = parser.parseFromString(text, "application/xml");
    const bucketNodes = xml.querySelectorAll("Bucket");
    
    const buckets = [];
    bucketNodes.forEach(b => {
      const name = b.querySelector("Name")?.textContent;
      const creationDate = b.querySelector("CreationDate")?.textContent;
      if (name) {
        buckets.push({ name, creationDate });
      }
    });
    return buckets;
  }

  async createBucket(bucketName, options = {}) {
    const { url, headers } = await this.signRequest("PUT", `/${bucketName}`);
    const res = await fetch(url, { method: "PUT", headers });
    if (!res.ok) {
      const errText = await res.text();
      throw new Error(`Erro ao criar bucket: HTTP ${res.status} - ${errText}`);
    }

    // Se versionamento foi selecionado na criação
    if (options.versioning) {
      try {
        await this.putBucketVersioning(bucketName, "Enabled");
      } catch (vErr) {
        console.warn("Falha ao habilitar versionamento na criação:", vErr);
      }
    }
    return true;
  }

  async deleteBucket(bucketName) {
    const { url, headers } = await this.signRequest("DELETE", `/${bucketName}`);
    const res = await fetch(url, { method: "DELETE", headers });
    if (!res.ok) {
      const errText = await res.text();
      throw new Error(`Erro ao deletar bucket: HTTP ${res.status} - ${errText}`);
    }
    return true;
  }

  async getBucketVersioning(bucketName) {
    const { url, headers } = await this.signRequest("GET", `/${bucketName}`, { versioning: "" });
    const res = await fetch(url, { method: "GET", headers });
    if (!res.ok) return "Off";
    const text = await res.text();
    const parser = new DOMParser();
    const xml = parser.parseFromString(text, "application/xml");
    const status = xml.querySelector("Status")?.textContent;
    return status || "Off";
  }

  async putBucketVersioning(bucketName, status) {
    const xmlBody = `<VersioningConfiguration xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Status>${status}</Status></VersioningConfiguration>`;
    const { url, headers } = await this.signRequest("PUT", `/${bucketName}`, { versioning: "" }, xmlBody, {
      "content-type": "application/xml"
    });
    const res = await fetch(url, { method: "PUT", headers, body: xmlBody });
    if (!res.ok) {
      const errText = await res.text();
      throw new Error(`Erro ao atualizar versionamento: HTTP ${res.status} - ${errText}`);
    }
    return true;
  }

  async listObjects(bucketName, prefix = "", delimiter = "/") {
    const params = { "list-type": "2" };
    if (prefix) params["prefix"] = prefix;
    if (delimiter) params["delimiter"] = delimiter;

    const { url, headers } = await this.signRequest("GET", `/${bucketName}`, params);
    const res = await fetch(url, { method: "GET", headers });
    if (!res.ok) {
      throw new Error(`Erro ao listar objetos: HTTP ${res.status}`);
    }
    const text = await res.text();
    const parser = new DOMParser();
    const xml = parser.parseFromString(text, "application/xml");

    const folders = [];
    xml.querySelectorAll("CommonPrefixes > Prefix").forEach(p => {
      folders.push(p.textContent);
    });

    const objects = [];
    xml.querySelectorAll("Contents").forEach(c => {
      objects.push({
        key: c.querySelector("Key")?.textContent,
        lastModified: c.querySelector("LastModified")?.textContent,
        etag: c.querySelector("ETag")?.textContent?.replace(/"/g, ""),
        size: parseInt(c.querySelector("Size")?.textContent || "0", 10),
        storageClass: c.querySelector("StorageClass")?.textContent || "STANDARD"
      });
    });

    return { folders, objects };
  }

  async deleteObject(bucketName, key) {
    const { url, headers } = await this.signRequest("DELETE", `/${bucketName}/${encodeURIComponent(key)}`);
    const res = await fetch(url, { method: "DELETE", headers });
    if (!res.ok && res.status !== 204 && res.status !== 200) {
      throw new Error(`Erro ao excluir objeto: HTTP ${res.status}`);
    }
    return true;
  }

  async deleteObjects(bucketName, keys) {
    if (!keys || keys.length === 0) return true;
    let xmlBody = `<Delete><Quiet>true</Quiet>`;
    keys.forEach(k => {
      xmlBody += `<Object><Key>${k}</Key></Object>`;
    });
    xmlBody += `</Delete>`;

    const { url, headers } = await this.signRequest("POST", `/${bucketName}`, { delete: "" }, xmlBody, {
      "content-type": "application/xml"
    });
    const res = await fetch(url, { method: "POST", headers, body: xmlBody });
    if (!res.ok && res.status !== 200 && res.status !== 204) {
      throw new Error(`Erro ao excluir objetos em lote: HTTP ${res.status}`);
    }
    return true;
  }

  async emptyBucket(bucketName) {
    // List all objects recursively without delimiter
    const { objects } = await this.listObjects(bucketName, "", "");
    if (objects.length > 0) {
      const keys = objects.map(o => o.key);
      await this.deleteObjects(bucketName, keys);
    }
    return true;
  }
}

window.S3Client = S3Client;
