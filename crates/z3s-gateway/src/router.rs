use z3s_common::types::ByteRange;

/// Tipos de Ações S3 identificadas a partir de uma requisição HTTP
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum S3Action {
    ListBuckets,
    CreateBucket { bucket: String },
    DeleteBucket { bucket: String },
    HeadBucket { bucket: String },
    GetBucketLocation { bucket: String },
    GetBucketVersioning { bucket: String },
    PutBucketVersioning { bucket: String },
    GetBucketAcl { bucket: String },
    PutBucketAcl { bucket: String },
    GetBucketPolicy { bucket: String },
    PutBucketPolicy { bucket: String },
    DeleteBucketPolicy { bucket: String },
    GetBucketCors { bucket: String },
    GetBucketLifecycle { bucket: String },
    PutBucketLifecycle { bucket: String },
    DeleteBucketLifecycle { bucket: String },
    GetBucketTagging { bucket: String },
    GetBucketEncryption { bucket: String },
    PutBucketEncryption { bucket: String },
    DeleteBucketEncryption { bucket: String },
    GetPublicAccessBlock { bucket: String },
    ListMultipartUploads { bucket: String },
    ListObjectsV2 { bucket: String, prefix: String, delimiter: Option<String>, max_keys: usize },
    ListObjectVersions {
        bucket: String,
        prefix: String,
        delimiter: Option<String>,
        key_marker: Option<String>,
        version_id_marker: Option<String>,
        max_keys: usize,
    },
    PutObject { bucket: String, key: String },
    CopyObject { bucket: String, key: String, source_bucket: String, source_key: String },
    GetObject { bucket: String, key: String, version_id: Option<String>, range: Option<ByteRange> },
    GetObjectAcl { bucket: String, key: String },
    PutObjectAcl { bucket: String, key: String },
    HeadObject { bucket: String, key: String, version_id: Option<String> },
    DeleteObject { bucket: String, key: String, version_id: Option<String> },
    DeleteObjects { bucket: String },
    InitiateMultipartUpload { bucket: String, key: String },
    UploadPart { bucket: String, key: String, upload_id: String, part_number: u32 },
    CompleteMultipartUpload { bucket: String, key: String, upload_id: String },
    AbortMultipartUpload { bucket: String, key: String, upload_id: String },
}

/// Roteador que interpreta métodos HTTP, caminhos URI e query params para ações S3
pub struct S3Router;

impl S3Router {
    pub fn resolve(
        method: &str,
        path: &str,
        query: Option<&str>,
        headers: &std::collections::HashMap<String, String>,
    ) -> Option<S3Action> {
        let path = path.trim_start_matches('/');
        let mut segments: Vec<&str> = if path.is_empty() {
            Vec::new()
        } else {
            path.splitn(2, '/').collect()
        };

        // Se o segundo segmento for vazio (ex: "/meubucket/"), normaliza para operação em nível de bucket
        if segments.len() == 2 && segments[1].is_empty() {
            segments.pop();
        }

        match (method.to_uppercase().as_str(), segments.as_slice()) {
            // GET / -> ListBuckets
            ("GET", []) => Some(S3Action::ListBuckets),

            // Operações em Nível de Bucket
            ("HEAD", [bucket]) => Some(S3Action::HeadBucket {
                bucket: Self::url_decode(bucket),
            }),
            ("PUT", [bucket]) => {
                let bucket = Self::url_decode(bucket);
                if query.map_or(false, |q| Self::has_query_flag(q, "acl")) {
                    return Some(S3Action::PutBucketAcl {
                        bucket,
                    });
                }
                if query.map_or(false, |q| Self::has_query_flag(q, "versioning")) {
                    return Some(S3Action::PutBucketVersioning {
                        bucket,
                    });
                }
                if query.map_or(false, |q| Self::has_query_flag(q, "encryption")) {
                    return Some(S3Action::PutBucketEncryption {
                        bucket,
                    });
                }
                if query.map_or(false, |q| Self::has_query_flag(q, "policy")) {
                    return Some(S3Action::PutBucketPolicy {
                        bucket,
                    });
                }
                if query.map_or(false, |q| Self::has_query_flag(q, "lifecycle") || Self::has_query_flag(q, "lifecycleConfiguration")) {
                    return Some(S3Action::PutBucketLifecycle {
                        bucket,
                    });
                }
                Some(S3Action::CreateBucket {
                    bucket,
                })
            }
            ("DELETE", [bucket]) => {
                let bucket = Self::url_decode(bucket);
                if query.map_or(false, |q| Self::has_query_flag(q, "encryption")) {
                    return Some(S3Action::DeleteBucketEncryption {
                        bucket,
                    });
                }
                if query.map_or(false, |q| Self::has_query_flag(q, "policy")) {
                    return Some(S3Action::DeleteBucketPolicy {
                        bucket,
                    });
                }
                if query.map_or(false, |q| Self::has_query_flag(q, "lifecycle") || Self::has_query_flag(q, "lifecycleConfiguration")) {
                    return Some(S3Action::DeleteBucketLifecycle {
                        bucket,
                    });
                }
                Some(S3Action::DeleteBucket {
                    bucket,
                })
            }
            ("POST", [bucket]) => {
                let bucket = Self::url_decode(bucket);
                if query.map_or(false, |q| Self::has_query_flag(q, "delete")) {
                    return Some(S3Action::DeleteObjects {
                        bucket,
                    });
                }
                None
            }
            ("GET", [bucket]) => {
                let bucket = Self::url_decode(bucket);
                let mut is_location = false;
                let mut is_versioning = false;
                let mut is_versions = false;
                let mut is_acl = false;
                let mut is_policy = false;
                let mut is_cors = false;
                let mut is_lifecycle = false;
                let mut is_tagging = false;
                let mut is_encryption = false;
                let mut is_public_access_block = false;
                let mut is_uploads = false;

                let mut prefix = String::new();
                let mut delimiter = None;
                let mut key_marker = None;
                let mut version_id_marker = None;
                let mut max_keys = 1000;

                if let Some(q) = query {
                    for pair in q.split('&') {
                        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
                        match k {
                            "location" => is_location = true,
                            "versioning" => is_versioning = true,
                            "versions" => is_versions = true,
                            "acl" => is_acl = true,
                            "policy" => is_policy = true,
                            "cors" => is_cors = true,
                            "lifecycle" | "lifecycleConfiguration" => is_lifecycle = true,
                            "tagging" => is_tagging = true,
                            "encryption" => is_encryption = true,
                            "publicAccessBlock" => is_public_access_block = true,
                            "uploads" => is_uploads = true,
                            "prefix" => prefix = Self::url_decode(v),
                            "delimiter" => {
                                let d = Self::url_decode(v);
                                delimiter = if d.is_empty() { None } else { Some(d) };
                            }
                            "key-marker" | "keyMarker" => key_marker = Some(Self::url_decode(v)),
                            "version-id-marker" | "versionIdMarker" => version_id_marker = Some(Self::url_decode(v)),
                            "max-keys" | "maxKeys" => max_keys = v.parse().unwrap_or(1000),
                            _ => {}
                        }
                    }
                }

                if is_location {
                    Some(S3Action::GetBucketLocation {
                        bucket,
                    })
                } else if is_versioning {
                    Some(S3Action::GetBucketVersioning {
                        bucket,
                    })
                } else if is_versions {
                    Some(S3Action::ListObjectVersions {
                        bucket,
                        prefix,
                        delimiter,
                        key_marker,
                        version_id_marker,
                        max_keys,
                    })
                } else if is_acl {
                    Some(S3Action::GetBucketAcl {
                        bucket,
                    })
                } else if is_policy {
                    Some(S3Action::GetBucketPolicy {
                        bucket,
                    })
                } else if is_cors {
                    Some(S3Action::GetBucketCors {
                        bucket,
                    })
                } else if is_lifecycle {
                    Some(S3Action::GetBucketLifecycle {
                        bucket,
                    })
                } else if is_tagging {
                    Some(S3Action::GetBucketTagging {
                        bucket,
                    })
                } else if is_encryption {
                    Some(S3Action::GetBucketEncryption {
                        bucket,
                    })
                } else if is_public_access_block {
                    Some(S3Action::GetPublicAccessBlock {
                        bucket,
                    })
                } else if is_uploads {
                    Some(S3Action::ListMultipartUploads {
                        bucket,
                    })
                } else {
                    Some(S3Action::ListObjectsV2 {
                        bucket,
                        prefix,
                        delimiter,
                        max_keys,
                    })
                }
            }

            // Operações em Nível de Objeto
            ("HEAD", [bucket, raw_key]) => {
                let bucket = Self::url_decode(bucket);
                let key = Self::url_decode(raw_key);
                let mut version_id = None;

                if let Some(q) = query {
                    for pair in q.split('&') {
                        if let Some((k, v)) = pair.split_once('=') {
                            if k == "versionId" {
                                version_id = Some(Self::url_decode(v));
                            }
                        }
                    }
                }

                Some(S3Action::HeadObject {
                    bucket,
                    key,
                    version_id,
                })
            }
            ("PUT", [bucket, raw_key]) => {
                let bucket = Self::url_decode(bucket);
                let key = Self::url_decode(raw_key);

                if query.map_or(false, |q| Self::has_query_flag(q, "acl")) {
                    return Some(S3Action::PutObjectAcl {
                        bucket,
                        key,
                    });
                }

                // Checa se é CopyObject (cabeçalho x-amz-copy-source)
                if let Some(copy_source) = headers.get("x-amz-copy-source") {
                    let decoded_source = Self::url_decode(copy_source);
                    let clean_source = decoded_source.trim_start_matches('/');
                    let clean_source = clean_source.split_once('?').map(|(s, _)| s).unwrap_or(clean_source);
                    if let Some((src_bucket, src_key)) = clean_source.split_once('/') {
                        return Some(S3Action::CopyObject {
                            bucket,
                            key,
                            source_bucket: src_bucket.to_string(),
                            source_key: src_key.to_string(),
                        });
                    }
                }

                let mut upload_id = None;
                let mut part_number = None;

                if let Some(q) = query {
                    for pair in q.split('&') {
                        if let Some((k, v)) = pair.split_once('=') {
                            match k {
                                "uploadId" => upload_id = Some(v.to_string()),
                                "partNumber" => part_number = v.parse::<u32>().ok(),
                                _ => {}
                            }
                        }
                    }
                }

                if let (Some(uid), Some(pn)) = (upload_id, part_number) {
                    Some(S3Action::UploadPart {
                        bucket,
                        key,
                        upload_id: uid,
                        part_number: pn,
                    })
                } else {
                    Some(S3Action::PutObject {
                        bucket,
                        key,
                    })
                }
            }
            ("GET", [bucket, raw_key]) => {
                let bucket = Self::url_decode(bucket);
                let key = Self::url_decode(raw_key);

                if query.map_or(false, |q| Self::has_query_flag(q, "acl")) {
                    return Some(S3Action::GetObjectAcl {
                        bucket,
                        key,
                    });
                }

                let mut version_id = None;
                if let Some(q) = query {
                    for pair in q.split('&') {
                        if let Some((k, v)) = pair.split_once('=') {
                            if k == "versionId" {
                                version_id = Some(Self::url_decode(v));
                            }
                        }
                    }
                }

                let range = headers.get("range").or_else(|| headers.get("Range")).and_then(|h| Self::parse_byte_range(h));
                Some(S3Action::GetObject {
                    bucket,
                    key,
                    version_id,
                    range,
                })
            }
            ("POST", [bucket, raw_key]) => {
                let bucket = Self::url_decode(bucket);
                let key = Self::url_decode(raw_key);

                let is_initiate = query.map_or(false, |q| Self::has_query_flag(q, "uploads"));
                let mut upload_id = None;

                if let Some(q) = query {
                    for pair in q.split('&') {
                        if let Some((k, v)) = pair.split_once('=') {
                            if k == "uploadId" {
                                upload_id = Some(v.to_string());
                            }
                        }
                    }
                }

                if is_initiate {
                    Some(S3Action::InitiateMultipartUpload {
                        bucket,
                        key,
                    })
                } else if let Some(uid) = upload_id {
                    Some(S3Action::CompleteMultipartUpload {
                        bucket,
                        key,
                        upload_id: uid,
                    })
                } else {
                    None
                }
            }
            ("DELETE", [bucket, raw_key]) => {
                let bucket = Self::url_decode(bucket);
                let key = Self::url_decode(raw_key);

                let mut upload_id = None;
                let mut version_id = None;

                if let Some(q) = query {
                    for pair in q.split('&') {
                        if let Some((k, v)) = pair.split_once('=') {
                            if k == "uploadId" {
                                upload_id = Some(v.to_string());
                            } else if k == "versionId" {
                                version_id = Some(Self::url_decode(v));
                            }
                        }
                    }
                }

                if let Some(uid) = upload_id {
                    Some(S3Action::AbortMultipartUpload {
                        bucket,
                        key,
                        upload_id: uid,
                    })
                } else {
                    Some(S3Action::DeleteObject {
                        bucket,
                        key,
                        version_id,
                    })
                }
            }

            _ => None,
        }
    }

    /// Faz a decodificação de percent-encoding em parâmetros de URL (ex: %2F -> /)
    pub fn url_decode(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '%' {
                let h1 = chars.next().unwrap_or('0');
                let h2 = chars.next().unwrap_or('0');
                let hex_str = format!("{}{}", h1, h2);
                if let Ok(byte) = u8::from_str_radix(&hex_str, 16) {
                    result.push(byte as char);
                } else {
                    result.push('%');
                    result.push(h1);
                    result.push(h2);
                }
            } else if c == '+' {
                result.push(' ');
            } else {
                result.push(c);
            }
        }
        result
    }

    /// Faz o parsing do cabeçalho `Range: bytes=start-end`
    pub fn parse_byte_range(header: &str) -> Option<ByteRange> {
        let header = header.trim();
        if !header.starts_with("bytes=") {
            return None;
        }

        let range_str = &header["bytes=".len()..];
        let (start_str, end_str) = range_str.split_once('-')?;

        let start = start_str.parse::<u64>().ok()?;
        let end = if end_str.is_empty() {
            None
        } else {
            Some(end_str.parse::<u64>().ok()?)
        };

        Some(ByteRange { start, end })
    }

    /// Verifica de forma tolerante se uma flag/chave existe na query string (ex: "acl", "acl=", "delete")
    pub fn has_query_flag(query: &str, flag: &str) -> bool {
        for pair in query.split('&') {
            let key = pair.split_once('=').map(|(k, _)| k).unwrap_or(pair);
            if key.eq_ignore_ascii_case(flag) {
                return true;
            }
        }
        false
    }
}
