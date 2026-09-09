// --- Pure JS Universal Cryptographic Engine (Works in all HTTP / HTTPS / IP contexts) ---
const Z3SCrypto = {
  K: [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
  ],

  rotr(n, x) {
    return (x >>> n) | (x << (32 - n));
  },

  digest(data) {
    let bytes = data;
    if (typeof data === "string") {
      bytes = new TextEncoder().encode(data);
    } else if (data instanceof ArrayBuffer) {
      bytes = new Uint8Array(data);
    }
    const len = bytes.length;
    const bitLen = len * 8;
    const k = (56 - (len + 1) % 64 + 64) % 64;
    const totalLen = len + 1 + k + 8;
    const padded = new Uint8Array(totalLen);
    padded.set(bytes, 0);
    padded[len] = 0x80;

    const view = new DataView(padded.buffer);
    const highBits = Math.floor(bitLen / 0x100000000);
    const lowBits = bitLen >>> 0;
    view.setUint32(totalLen - 8, highBits, false);
    view.setUint32(totalLen - 4, lowBits, false);

    let h0 = 0x6a09e667, h1 = 0xbb67ae85, h2 = 0x3c6ef372, h3 = 0xa54ff53a;
    let h4 = 0x510e527f, h5 = 0x9b05688c, h6 = 0x1f83d9ab, h7 = 0x5be0cd19;

    const w = new Uint32Array(64);

    for (let chunk = 0; chunk < totalLen; chunk += 64) {
      for (let i = 0; i < 16; i++) {
        w[i] = view.getUint32(chunk + i * 4, false);
      }
      for (let i = 16; i < 64; i++) {
        const s0 = this.rotr(7, w[i - 15]) ^ this.rotr(18, w[i - 15]) ^ (w[i - 15] >>> 3);
        const s1 = this.rotr(17, w[i - 2]) ^ this.rotr(19, w[i - 2]) ^ (w[i - 2] >>> 10);
        w[i] = (((w[i - 16] + s0) | 0) + ((w[i - 7] + s1) | 0)) | 0;
      }

      let a = h0, b = h1, c = h2, d = h3, e = h4, f = h5, g = h6, h = h7;

      for (let i = 0; i < 64; i++) {
        const S1 = this.rotr(6, e) ^ this.rotr(11, e) ^ this.rotr(25, e);
        const ch = (e & f) ^ ((~e) & g);
        const temp1 = (((((h + S1) | 0) + ch) | 0) + ((this.K[i] + w[i]) | 0)) | 0;
        const S0 = this.rotr(2, a) ^ this.rotr(13, a) ^ this.rotr(22, a);
        const maj = (a & b) ^ (a & c) ^ (b & c);
        const temp2 = (S0 + maj) | 0;

        h = g;
        g = f;
        f = e;
        e = (d + temp1) | 0;
        d = c;
        c = b;
        b = a;
        a = (temp1 + temp2) | 0;
      }

      h0 = (h0 + a) | 0;
      h1 = (h1 + b) | 0;
      h2 = (h2 + c) | 0;
      h3 = (h3 + d) | 0;
      h4 = (h4 + e) | 0;
      h5 = (h5 + f) | 0;
      h6 = (h6 + g) | 0;
      h7 = (h7 + h) | 0;
    }

    const out = new Uint8Array(32);
    const outView = new DataView(out.buffer);
    outView.setUint32(0, h0, false);
    outView.setUint32(4, h1, false);
    outView.setUint32(8, h2, false);
    outView.setUint32(12, h3, false);
    outView.setUint32(16, h4, false);
    outView.setUint32(20, h5, false);
    outView.setUint32(24, h6, false);
    outView.setUint32(28, h7, false);
    return out;
  },

  hmac(key, data) {
    let kBytes = typeof key === "string" ? new TextEncoder().encode(key) : key;
    let dBytes = typeof data === "string" ? new TextEncoder().encode(data) : data;

    let k = new Uint8Array(kBytes);
    if (k.length > 64) {
      k = this.digest(k);
    }
    const paddedKey = new Uint8Array(64);
    paddedKey.set(k, 0);

    const ipad = new Uint8Array(64 + dBytes.length);
    const opad = new Uint8Array(64 + 32);

    for (let i = 0; i < 64; i++) {
      ipad[i] = paddedKey[i] ^ 0x36;
      opad[i] = paddedKey[i] ^ 0x5c;
    }
    ipad.set(dBytes, 64);

    const innerHash = this.digest(ipad);
    opad.set(innerHash, 64);

    return this.digest(opad);
  },

  toHex(u8) {
    return Array.from(u8).map(b => b.toString(16).padStart(2, "0")).join("");
  }
};

class S3Client {
  constructor(endpoint, accessKey, secretKey, region = "us-east-1") {
    const session = typeof AuthManager !== "undefined" && AuthManager.getSession ? AuthManager.getSession() : null;
    const ep = endpoint || session?.endpoint || (typeof window !== "undefined" && window.location?.origin ? window.location.origin : "http://localhost:9000");
    this.endpoint = (ep || "").replace(/\/$/, "");
    this.accessKey = accessKey || session?.accessKey || "";
    this.secretKey = secretKey || session?.secretKey || "";
    this.region = region || session?.region || "us-east-1";
    this.service = "s3";
  }

  // --- Universal Crypto Helpers ---
  async sha256(data) {
    return Z3SCrypto.toHex(Z3SCrypto.digest(data));
  }

  async hmacRaw(key, data) {
    return Z3SCrypto.hmac(key, data);
  }

  async getSignatureKey(key, dateStamp, regionName, serviceName) {
    const kDate = await this.hmacRaw("AWS4" + key, dateStamp);
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

  async listObjectVersions(bucketName, prefix = "", delimiter = "/") {
    const params = { versions: "" };
    if (prefix) params["prefix"] = prefix;
    if (delimiter) params["delimiter"] = delimiter;

    const { url, headers } = await this.signRequest("GET", `/${bucketName}`, params);
    const res = await fetch(url, { method: "GET", headers });
    if (!res.ok) {
      throw new Error(`Erro ao listar versões: HTTP ${res.status}`);
    }
    const text = await res.text();
    const parser = new DOMParser();
    const xml = parser.parseFromString(text, "application/xml");

    const folders = [];
    xml.querySelectorAll("CommonPrefixes > Prefix").forEach(p => {
      folders.push(p.textContent);
    });

    const versions = [];
    xml.querySelectorAll("Version").forEach(v => {
      versions.push({
        key: v.querySelector("Key")?.textContent,
        versionId: v.querySelector("VersionId")?.textContent,
        isLatest: v.querySelector("IsLatest")?.textContent === "true",
        lastModified: v.querySelector("LastModified")?.textContent,
        etag: v.querySelector("ETag")?.textContent?.replace(/"/g, ""),
        size: parseInt(v.querySelector("Size")?.textContent || "0", 10),
        storageClass: v.querySelector("StorageClass")?.textContent || "STANDARD",
        isDeleteMarker: false
      });
    });

    const deleteMarkers = [];
    xml.querySelectorAll("DeleteMarker").forEach(d => {
      deleteMarkers.push({
        key: d.querySelector("Key")?.textContent,
        versionId: d.querySelector("VersionId")?.textContent,
        isLatest: d.querySelector("IsLatest")?.textContent === "true",
        lastModified: d.querySelector("LastModified")?.textContent,
        isDeleteMarker: true
      });
    });

    return { folders, versions, deleteMarkers };
  }

  async createFolder(bucketName, folderKey) {
    let cleanKey = folderKey.trim();
    if (!cleanKey.endsWith("/")) cleanKey += "/";
    if (cleanKey.startsWith("/")) cleanKey = cleanKey.substring(1);

    const { url, headers } = await this.signRequest("PUT", `/${bucketName}/${cleanKey}`, {}, "", {
      "content-type": "application/x-directory"
    });
    const res = await fetch(url, { method: "PUT", headers, body: "" });
    if (!res.ok && res.status !== 200 && res.status !== 204) {
      const errText = await res.text();
      throw new Error(`Erro ao criar pasta: HTTP ${res.status} - ${errText}`);
    }
    return true;
  }

  async uploadObject(bucketName, key, fileOrBlob, contentType = "application/octet-stream", userMetadata = {}, onProgress = null) {
    let cleanKey = key.trim();
    if (cleanKey.startsWith("/")) cleanKey = cleanKey.substring(1);

    // Se arquivo for grande (> 15MB), utiliza Multipart Upload
    const MULTIPART_THRESHOLD = 15 * 1024 * 1024;
    if (fileOrBlob.size > MULTIPART_THRESHOLD) {
      return this.uploadMultipart(bucketName, cleanKey, fileOrBlob, contentType, userMetadata, onProgress);
    }

    // Upload direto (Single PUT) com suporte a progresso via XHR
    const customHeaders = {
      "content-type": contentType || "application/octet-stream"
    };
    for (const [mk, mv] of Object.entries(userMetadata)) {
      if (mk && mv) {
        const headerKey = mk.startsWith("x-amz-meta-") ? mk : `x-amz-meta-${mk}`;
        customHeaders[headerKey] = String(mv);
      }
    }

    const buffer = await fileOrBlob.arrayBuffer();
    const uint8 = new Uint8Array(buffer);
    const { url, headers } = await this.signRequest("PUT", `/${bucketName}/${cleanKey}`, {}, uint8, customHeaders);

    return new Promise((resolve, reject) => {
      const xhr = new XMLHttpRequest();
      xhr.open("PUT", url, true);

      for (const [hk, hv] of Object.entries(headers)) {
        xhr.setRequestHeader(hk, hv);
      }

      if (xhr.upload && onProgress) {
        xhr.upload.onprogress = (event) => {
          if (event.lengthComputable) {
            const percent = Math.round((event.loaded / event.total) * 100);
            onProgress({ loaded: event.loaded, total: event.total, percent });
          }
        };
      }

      xhr.onload = () => {
        if (xhr.status >= 200 && xhr.status < 300) {
          if (onProgress) onProgress({ loaded: fileOrBlob.size, total: fileOrBlob.size, percent: 100 });
          resolve(true);
        } else {
          reject(new Error(`Erro no upload de '${cleanKey}': HTTP ${xhr.status} - ${xhr.responseText}`));
        }
      };

      xhr.onerror = () => {
        reject(new Error(`Erro de rede durante o upload de '${cleanKey}'`));
      };

      xhr.send(uint8);
    });
  }

  async uploadMultipart(bucketName, key, fileOrBlob, contentType = "application/octet-stream", userMetadata = {}, onProgress = null) {
    const uploadId = await this.initiateMultipartUpload(bucketName, key, contentType, userMetadata);
    const PART_SIZE = 8 * 1024 * 1024; // 8MB por parte
    const totalSize = fileOrBlob.size;
    const numParts = Math.ceil(totalSize / PART_SIZE);
    const parts = [];
    let uploadedBytes = 0;

    try {
      for (let partNumber = 1; partNumber <= numParts; partNumber++) {
        const start = (partNumber - 1) * PART_SIZE;
        const end = Math.min(start + PART_SIZE, totalSize);
        const chunk = fileOrBlob.slice(start, end);
        const chunkBuffer = await chunk.arrayBuffer();
        const chunkUint8 = new Uint8Array(chunkBuffer);

        const etag = await this.uploadPart(bucketName, key, uploadId, partNumber, chunkUint8, (partProg) => {
          if (onProgress) {
            const currentTotal = uploadedBytes + partProg.loaded;
            const percent = Math.min(99, Math.round((currentTotal / totalSize) * 100));
            onProgress({ loaded: currentTotal, total: totalSize, percent });
          }
        });

        parts.push({ partNumber, etag });
        uploadedBytes += chunk.size;
      }

      await this.completeMultipartUpload(bucketName, key, uploadId, parts);
      if (onProgress) onProgress({ loaded: totalSize, total: totalSize, percent: 100 });
      return true;
    } catch (err) {
      console.error("Multipart upload error, aborting:", err);
      try {
        await this.abortMultipartUpload(bucketName, key, uploadId);
      } catch (abortErr) {
        console.warn("Falha ao abortar multipart:", abortErr);
      }
      throw err;
    }
  }

  async initiateMultipartUpload(bucketName, key, contentType = "application/octet-stream", userMetadata = {}) {
    const customHeaders = { "content-type": contentType };
    for (const [mk, mv] of Object.entries(userMetadata)) {
      if (mk && mv) customHeaders[`x-amz-meta-${mk}`] = String(mv);
    }
    const { url, headers } = await this.signRequest("POST", `/${bucketName}/${key}`, { uploads: "" }, "", customHeaders);
    const res = await fetch(url, { method: "POST", headers });
    if (!res.ok) {
      throw new Error(`Erro ao iniciar multipart upload: HTTP ${res.status}`);
    }
    const text = await res.text();
    const parser = new DOMParser();
    const xml = parser.parseFromString(text, "application/xml");
    const uploadId = xml.querySelector("UploadId")?.textContent;
    if (!uploadId) throw new Error("UploadId não retornado pelo servidor");
    return uploadId;
  }

  async uploadPart(bucketName, key, uploadId, partNumber, dataUint8, onProgress = null) {
    const params = { partNumber: String(partNumber), uploadId: uploadId };
    const { url, headers } = await this.signRequest("PUT", `/${bucketName}/${key}`, params, dataUint8);

    return new Promise((resolve, reject) => {
      const xhr = new XMLHttpRequest();
      xhr.open("PUT", url, true);
      for (const [hk, hv] of Object.entries(headers)) {
        xhr.setRequestHeader(hk, hv);
      }
      if (xhr.upload && onProgress) {
        xhr.upload.onprogress = (e) => {
          if (e.lengthComputable) onProgress({ loaded: e.loaded, total: e.total });
        };
      }
      xhr.onload = () => {
        if (xhr.status >= 200 && xhr.status < 300) {
          const etag = xhr.getResponseHeader("ETag") || `"${partNumber}"`;
          resolve(etag.replace(/"/g, ""));
        } else {
          reject(new Error(`Erro ao enviar parte ${partNumber}: HTTP ${xhr.status}`));
        }
      };
      xhr.onerror = () => reject(new Error(`Erro de rede na parte ${partNumber}`));
      xhr.send(dataUint8);
    });
  }

  async completeMultipartUpload(bucketName, key, uploadId, parts) {
    let xml = `<CompleteMultipartUpload>`;
    for (const p of parts) {
      xml += `<Part><PartNumber>${p.partNumber}</PartNumber><ETag>"${p.etag}"</ETag></Part>`;
    }
    xml += `</CompleteMultipartUpload>`;

    const { url, headers } = await this.signRequest("POST", `/${bucketName}/${key}`, { uploadId }, xml, {
      "content-type": "application/xml"
    });
    const res = await fetch(url, { method: "POST", headers, body: xml });
    if (!res.ok) {
      const errText = await res.text();
      throw new Error(`Erro ao finalizar multipart upload: HTTP ${res.status} - ${errText}`);
    }
    return true;
  }

  async abortMultipartUpload(bucketName, key, uploadId) {
    const { url, headers } = await this.signRequest("DELETE", `/${bucketName}/${key}`, { uploadId });
    await fetch(url, { method: "DELETE", headers });
    return true;
  }

  async downloadObject(bucketName, key) {
    const { url, headers } = await this.signRequest("GET", `/${bucketName}/${encodeURI(key)}`);
    const res = await fetch(url, { method: "GET", headers });
    if (!res.ok) throw new Error(`Erro ao baixar objeto: HTTP ${res.status}`);

    const blob = await res.blob();
    const blobUrl = URL.createObjectURL(blob);
    const filename = key.split("/").filter(Boolean).pop() || "download";

    const a = document.createElement("a");
    a.href = blobUrl;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    setTimeout(() => URL.revokeObjectURL(blobUrl), 10000);
    return true;
  }

  async getObjectBlob(bucketName, key) {
    const { url, headers } = await this.signRequest("GET", `/${bucketName}/${encodeURI(key)}`);
    const res = await fetch(url, { method: "GET", headers });
    if (!res.ok) throw new Error(`Erro ao buscar objeto: HTTP ${res.status}`);

    const contentType = res.headers.get("content-type") || "application/octet-stream";
    const blob = await res.blob();
    return {
      blob,
      contentType,
      size: blob.size,
      etag: (res.headers.get("etag") || "").replace(/"/g, ""),
      lastModified: res.headers.get("last-modified")
    };
  }

  async deleteObject(bucketName, key) {
    const { url, headers } = await this.signRequest("DELETE", `/${bucketName}/${encodeURIComponent(key)}`);
    const res = await fetch(url, { method: "DELETE", headers });
    if (!res.ok && res.status !== 204 && res.status !== 200) {
      throw new Error(`Erro ao excluir objeto: HTTP ${res.status}`);
    }
    return true;
  }

  async deleteObjectVersion(bucketName, key, versionId) {
    const { url, headers } = await this.signRequest("DELETE", `/${bucketName}/${encodeURIComponent(key)}`, { versionId });
    const res = await fetch(url, { method: "DELETE", headers });
    if (!res.ok && res.status !== 204 && res.status !== 200) {
      throw new Error(`Erro ao excluir versão do objeto: HTTP ${res.status}`);
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

  // --- Bucket Policy, CORS & Lifecycle (Opção 1) ---

  async getBucketPolicy(bucketName) {
    const { url, headers } = await this.signRequest("GET", `/${bucketName}`, { policy: "" });
    const res = await fetch(url, { method: "GET", headers });
    if (!res.ok) {
      if (res.status === 404) return null;
      throw new Error(`Erro ao obter bucket policy: HTTP ${res.status}`);
    }
    return await res.text();
  }

  async putBucketPolicy(bucketName, policyJson) {
    const { url, headers } = await this.signRequest("PUT", `/${bucketName}`, { policy: "" }, policyJson, {
      "content-type": "application/json"
    });
    const res = await fetch(url, { method: "PUT", headers, body: policyJson });
    if (!res.ok) {
      const errText = await res.text();
      throw new Error(`Erro ao salvar bucket policy: HTTP ${res.status} - ${errText}`);
    }
    return true;
  }

  async deleteBucketPolicy(bucketName) {
    const { url, headers } = await this.signRequest("DELETE", `/${bucketName}`, { policy: "" });
    const res = await fetch(url, { method: "DELETE", headers });
    if (!res.ok && res.status !== 204 && res.status !== 200) {
      throw new Error(`Erro ao excluir bucket policy: HTTP ${res.status}`);
    }
    return true;
  }

  async getBucketCors(bucketName) {
    const { url, headers } = await this.signRequest("GET", `/${bucketName}`, { cors: "" });
    const res = await fetch(url, { method: "GET", headers });
    if (!res.ok) {
      if (res.status === 404) return null;
      return null;
    }
    return await res.text();
  }

  async putBucketCors(bucketName, corsXmlOrJson) {
    const isXml = corsXmlOrJson.trim().startsWith("<");
    const contentType = isXml ? "application/xml" : "application/json";
    const { url, headers } = await this.signRequest("PUT", `/${bucketName}`, { cors: "" }, corsXmlOrJson, {
      "content-type": contentType
    });
    const res = await fetch(url, { method: "PUT", headers, body: corsXmlOrJson });
    if (!res.ok) {
      const errText = await res.text();
      throw new Error(`Erro ao salvar CORS: HTTP ${res.status} - ${errText}`);
    }
    return true;
  }

  async deleteBucketCors(bucketName) {
    const { url, headers } = await this.signRequest("DELETE", `/${bucketName}`, { cors: "" });
    const res = await fetch(url, { method: "DELETE", headers });
    if (!res.ok && res.status !== 204 && res.status !== 200) {
      throw new Error(`Erro ao excluir CORS: HTTP ${res.status}`);
    }
    return true;
  }

  async getBucketLifecycle(bucketName) {
    const { url, headers } = await this.signRequest("GET", `/${bucketName}`, { lifecycle: "" });
    const res = await fetch(url, { method: "GET", headers });
    if (!res.ok) return null;
    return await res.text();
  }

  async putBucketLifecycle(bucketName, lifecycleXml) {
    const { url, headers } = await this.signRequest("PUT", `/${bucketName}`, { lifecycle: "" }, lifecycleXml, {
      "content-type": "application/xml"
    });
    const res = await fetch(url, { method: "PUT", headers, body: lifecycleXml });
    if (!res.ok) {
      const errText = await res.text();
      throw new Error(`Erro ao salvar lifecycle: HTTP ${res.status} - ${errText}`);
    }
    return true;
  }

  async deleteBucketLifecycle(bucketName) {
    const { url, headers } = await this.signRequest("DELETE", `/${bucketName}`, { lifecycle: "" });
    const res = await fetch(url, { method: "DELETE", headers });
    if (!res.ok && res.status !== 204 && res.status !== 200) {
      throw new Error(`Erro ao excluir lifecycle: HTTP ${res.status}`);
    }
    return true;
  }

  // --- Cluster Metrics & IAM Management (Opções 2 & 3) ---

  async getClusterMetrics() {
    const res = await fetch(`${this.endpoint}/z3s/api/cluster/metrics`);
    if (!res.ok) throw new Error(`Erro ao obter métricas: HTTP ${res.status}`);
    return await res.json();
  }

  async getReplicationStatus() {
    const res = await fetch(`${this.endpoint}/z3s/api/cluster/replication`);
    if (!res.ok) throw new Error(`Erro ao obter status de replicação: HTTP ${res.status}`);
    return await res.json();
  }

  async saveReplicationConfig(config) {
    const res = await fetch(`${this.endpoint}/z3s/api/cluster/replication`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(config),
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data.error || `Erro ao salvar replicação: HTTP ${res.status}`);
    return data;
  }

  async testReplicationConfig(config) {
    const res = await fetch(`${this.endpoint}/z3s/api/cluster/replication/test`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(config),
    });
    const data = await res.json();
    if (!res.ok || data.ok === false) {
      throw new Error(data.error || `Falha no teste: HTTP ${res.status}`);
    }
    return data;
  }

  async syncReplication() {
    const res = await fetch(`${this.endpoint}/z3s/api/cluster/replication/sync`, {
      method: "POST"
    });
    const data = await res.json();
    if (!res.ok || data.ok === false) {
      throw new Error(data.error || `Falha na sincronização: HTTP ${res.status}`);
    }
    return data;
  }

  async listAccessKeys() {
    const res = await fetch(`${this.endpoint}/z3s/api/iam/keys`);
    if (!res.ok) throw new Error(`Erro ao listar chaves IAM: HTTP ${res.status}`);
    const data = await res.json();
    return data.keys || [];
  }

  async createAccessKey() {
    const res = await fetch(`${this.endpoint}/z3s/api/iam/keys`, { method: "POST" });
    if (!res.ok) throw new Error(`Erro ao criar chave IAM: HTTP ${res.status}`);
    return await res.json();
  }

  async deleteAccessKey(accessKeyId) {
    const res = await fetch(`${this.endpoint}/z3s/api/iam/keys?access_key_id=${encodeURIComponent(accessKeyId)}`, {
      method: "DELETE"
    });
    if (!res.ok) throw new Error(`Erro ao excluir chave IAM: HTTP ${res.status}`);
    return true;
  }
}

window.S3Client = S3Client;
