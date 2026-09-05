use crate::router::{S3Action, S3Router};
use crate::xml::{
    AccessControlPolicy, BucketItem, CommonPrefixItem, CompleteMultipartUploadResult,
    CopyObjectResult, DeleteMarkerItem, DeletedItem, DeleteResult, InitiateMultipartUploadResult,
    ListAllMyBucketsResult, ListBucketResult, ListMultipartUploadsResult, ListVersionsResult,
    LocationConstraint, ObjectItem, S3XmlError, VersionItem, VersioningConfiguration,
};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, RwLock};
use uuid::Uuid;
use z3s_auth::credentials::CredentialsProvider;
use z3s_auth::sigv4::{SigV4Engine, UNSIGNED_PAYLOAD};
use z3s_common::hash::{blake3_hash, calculate_md5_etag};
use z3s_common::manifest::{ObjectManifest, ObjectMetadata, PartManifest, ShardPointer, StorageClass};
use z3s_common::types::{BucketName, ByteRange, ETag, ObjectKey, ShardId, VersionId};
use z3s_common::S3ErrorCode;
use z3s_erasure::ErasureEngine;
use z3s_storage::StorageEngine;

/// Resposta HTTP de alto nível gerada pelo Gateway
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayHttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
}

impl GatewayHttpResponse {
    pub fn ok_xml(xml: String) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/xml".to_string());
        headers.insert("Content-Length".to_string(), xml.len().to_string());
        Self {
            status: 200,
            headers,
            body: Bytes::from(xml),
        }
    }

    pub fn ok_empty() -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Length".to_string(), "0".to_string());
        Self {
            status: 200,
            headers,
            body: Bytes::new(),
        }
    }

    pub fn ok_bytes(data: Vec<u8>, content_type: &str, etag: &ETag) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), content_type.to_string());
        headers.insert("Content-Length".to_string(), data.len().to_string());
        headers.insert("ETag".to_string(), etag.as_str().to_string());
        headers.insert("Accept-Ranges".to_string(), "bytes".to_string());
        Self {
            status: 200,
            headers,
            body: Bytes::from(data),
        }
    }

    pub fn partial_content(data: Vec<u8>, start: u64, end: u64, total: u64, content_type: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), content_type.to_string());
        headers.insert("Content-Length".to_string(), data.len().to_string());
        headers.insert("Accept-Ranges".to_string(), "bytes".to_string());
        headers.insert(
            "Content-Range".to_string(),
            format!("bytes {}-{}/{}", start, end, total),
        );
        Self {
            status: 206,
            headers,
            body: Bytes::from(data),
        }
    }

    pub fn error(code: S3ErrorCode, message: impl Into<String>, resource: Option<String>) -> Self {
        let err_xml = S3XmlError::new(code, message, resource).to_xml();
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/xml".to_string());
        headers.insert("Content-Length".to_string(), err_xml.len().to_string());
        Self {
            status: code.http_status(),
            headers,
            body: Bytes::from(err_xml),
        }
    }
}

/// Estado do Versionamento de um Bucket
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BucketVersioningStatus {
    Off,
    Enabled,
    Suspended,
}

impl Default for BucketVersioningStatus {
    fn default() -> Self {
        Self::Off
    }
}

/// Metadados de um Bucket no Z3S
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketInfo {
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub versioning: BucketVersioningStatus,
}

/// Estado em andamento de um Multipart Upload
#[derive(Debug, Clone)]
struct ActiveMultipartUpload {
    parts: HashMap<u32, PartManifest>,
}

/// Serviço Central do S3 Gateway
pub struct S3GatewayService {
    pub node_id: Uuid,
    pub storage: Arc<StorageEngine>,
    pub erasure: Arc<ErasureEngine>,
    pub credentials: Arc<dyn CredentialsProvider>,
    pub metadata_dir: Option<std::path::PathBuf>,
    buckets: RwLock<HashMap<String, BucketInfo>>,
    objects: RwLock<HashMap<String, Vec<ObjectManifest>>>,
    multiparts: RwLock<HashMap<String, ActiveMultipartUpload>>,
}

impl S3GatewayService {
    pub fn new(
        node_id: Uuid,
        storage: Arc<StorageEngine>,
        erasure: Arc<ErasureEngine>,
        credentials: Arc<dyn CredentialsProvider>,
    ) -> Self {
        Self::new_with_metadata(node_id, storage, erasure, credentials, None)
    }

    pub fn new_with_metadata(
        node_id: Uuid,
        storage: Arc<StorageEngine>,
        erasure: Arc<ErasureEngine>,
        credentials: Arc<dyn CredentialsProvider>,
        metadata_dir: Option<std::path::PathBuf>,
    ) -> Self {
        let mut buckets = HashMap::new();
        let mut objects = HashMap::new();

        if let Some(ref dir) = metadata_dir {
            let _ = std::fs::create_dir_all(dir);
            let buckets_file = dir.join("buckets.json");
            if buckets_file.exists() {
                if let Ok(data) = std::fs::read_to_string(&buckets_file) {
                    if let Ok(loaded) = serde_json::from_str::<HashMap<String, BucketInfo>>(&data) {
                        buckets = loaded;
                    } else if let Ok(old_buckets) = serde_json::from_str::<HashMap<String, chrono::DateTime<chrono::Utc>>>(&data) {
                        for (k, v) in old_buckets {
                            buckets.insert(k.clone(), BucketInfo {
                                name: k,
                                created_at: v,
                                versioning: BucketVersioningStatus::Off,
                            });
                        }
                    }
                }
            }
            let objects_file = dir.join("objects.json");
            if objects_file.exists() {
                if let Ok(data) = std::fs::read_to_string(&objects_file) {
                    if let Ok(loaded) = serde_json::from_str::<HashMap<String, Vec<ObjectManifest>>>(&data) {
                        objects = loaded;
                    } else if let Ok(old_objects) = serde_json::from_str::<HashMap<String, ObjectManifest>>(&data) {
                        for (k, v) in old_objects {
                            objects.insert(k, vec![v]);
                        }
                    }
                }
            }
        }

        Self {
            node_id,
            storage,
            erasure,
            credentials,
            metadata_dir,
            buckets: RwLock::new(buckets),
            objects: RwLock::new(objects),
            multiparts: RwLock::new(HashMap::new()),
        }
    }

    pub fn persist_buckets(&self) {
        if let Some(ref dir) = self.metadata_dir {
            let map = self.buckets.read().unwrap();
            if let Ok(json) = serde_json::to_string_pretty(&*map) {
                let _ = std::fs::create_dir_all(dir);
                let _ = std::fs::write(dir.join("buckets.json"), json);
            }
        }
    }

    pub fn persist_objects(&self) {
        if let Some(ref dir) = self.metadata_dir {
            let map = self.objects.read().unwrap();
            if let Ok(json) = serde_json::to_string_pretty(&*map) {
                let _ = std::fs::create_dir_all(dir);
                let _ = std::fs::write(dir.join("objects.json"), json);
            }
        }
    }

    /// Valida a assinatura SigV4 se o cabeçalho Authorization estiver presente
    pub fn authenticate(
        &self,
        method: &str,
        path: &str,
        query: Option<&str>,
        headers: &HashMap<String, String>,
    ) -> Result<(), S3ErrorCode> {
        if let Some(auth_header) = headers.get("authorization") {
            let auth_context = match SigV4Engine::parse_authorization_header(auth_header) {
                Ok(ctx) => ctx,
                Err(_) => return Err(S3ErrorCode::InvalidArgument),
            };

            let timestamp = headers
                .get("x-amz-date")
                .or_else(|| headers.get("date"))
                .cloned()
                .unwrap_or_default();

            let payload_hash = headers
                .get("x-amz-content-sha256")
                .cloned()
                .unwrap_or_else(|| UNSIGNED_PAYLOAD.to_string());

            let mut btree_headers = BTreeMap::new();
            for (k, v) in headers {
                btree_headers.insert(k.to_ascii_lowercase(), v.clone());
            }

            SigV4Engine::verify(
                &auth_context,
                self.credentials.as_ref(),
                method,
                path,
                query.unwrap_or(""),
                &btree_headers,
                &payload_hash,
                &timestamp,
            )
            .map_err(|_| S3ErrorCode::SignatureDoesNotMatch)?;
        }

        Ok(())
    }

    /// Processa uma requisição HTTP REST completa
    pub fn handle_request(
        &self,
        method: &str,
        path: &str,
        query: Option<&str>,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> GatewayHttpResponse {
        if let Err(err_code) = self.authenticate(method, path, query, headers) {
            return GatewayHttpResponse::error(
                err_code,
                "A assinatura calculada da requisição não confere ou credenciais inválidas",
                Some(path.to_string()),
            );
        }

        let action = match S3Router::resolve(method, path, query, headers) {
            Some(a) => a,
            None => {
                return GatewayHttpResponse::error(
                    S3ErrorCode::InvalidArgument,
                    "Rota ou método HTTP não suportado",
                    Some(path.to_string()),
                )
            }
        };

        match action {
            S3Action::ListBuckets => self.handle_list_buckets(),
            S3Action::HeadBucket { bucket } => self.handle_head_bucket(&bucket),
            S3Action::GetBucketLocation { bucket } => self.handle_get_bucket_location(&bucket),
            S3Action::GetBucketVersioning { bucket } => self.handle_get_bucket_versioning(&bucket),
            S3Action::PutBucketVersioning { bucket } => self.handle_put_bucket_versioning(&bucket, body),
            S3Action::GetBucketAcl { bucket } => self.handle_get_bucket_acl(&bucket),
            S3Action::PutBucketAcl { bucket } => self.handle_put_bucket_acl(&bucket),
            S3Action::GetBucketPolicy { bucket } => self.handle_get_bucket_policy(&bucket),
            S3Action::GetBucketCors { bucket } => self.handle_get_bucket_cors(&bucket),
            S3Action::GetBucketLifecycle { bucket } => self.handle_get_bucket_lifecycle(&bucket),
            S3Action::GetBucketTagging { bucket } => self.handle_get_bucket_tagging(&bucket),
            S3Action::GetBucketEncryption { bucket } => self.handle_get_bucket_encryption(&bucket),
            S3Action::GetPublicAccessBlock { bucket } => self.handle_get_public_access_block(&bucket),
            S3Action::ListMultipartUploads { bucket } => self.handle_list_multipart_uploads(&bucket),
            S3Action::CreateBucket { bucket } => self.handle_create_bucket(&bucket),
            S3Action::DeleteBucket { bucket } => self.handle_delete_bucket(&bucket),
            S3Action::ListObjectsV2 {
                bucket,
                prefix,
                delimiter,
                max_keys,
            } => self.handle_list_objects(&bucket, &prefix, delimiter, max_keys),
            S3Action::ListObjectVersions {
                bucket,
                prefix,
                delimiter,
                key_marker,
                version_id_marker,
                max_keys,
            } => self.handle_list_object_versions(&bucket, &prefix, delimiter, key_marker, version_id_marker, max_keys),
            S3Action::PutObject { bucket, key } => self.handle_put_object(&bucket, &key, headers, body),
            S3Action::CopyObject {
                bucket,
                key,
                source_bucket,
                source_key,
            } => self.handle_copy_object(&bucket, &key, &source_bucket, &source_key, headers),
            S3Action::GetObject { bucket, key, version_id, range } => {
                self.handle_get_object(&bucket, &key, version_id.as_deref(), range, headers)
            }
            S3Action::GetObjectAcl { bucket, key } => self.handle_get_object_acl(&bucket, &key),
            S3Action::PutObjectAcl { bucket, key } => self.handle_put_object_acl(&bucket, &key),
            S3Action::HeadObject { bucket, key, version_id } => {
                self.handle_head_object(&bucket, &key, version_id.as_deref())
            }
            S3Action::DeleteObject { bucket, key, version_id } => {
                self.handle_delete_object(&bucket, &key, version_id.as_deref())
            }
            S3Action::DeleteObjects { bucket } => self.handle_delete_objects(&bucket, body),
            S3Action::InitiateMultipartUpload { bucket, key } => {
                self.handle_initiate_multipart(&bucket, &key)
            }
            S3Action::UploadPart {
                bucket,
                key,
                upload_id,
                part_number,
            } => self.handle_upload_part(&bucket, &key, &upload_id, part_number, body),
            S3Action::CompleteMultipartUpload {
                bucket,
                key,
                upload_id,
            } => self.handle_complete_multipart(&bucket, &key, &upload_id, headers),
            S3Action::AbortMultipartUpload {
                bucket,
                key,
                upload_id,
            } => self.handle_abort_multipart(&bucket, &key, &upload_id),
        }
    }

    fn handle_list_buckets(&self) -> GatewayHttpResponse {
        let buckets = self.buckets.read().unwrap();
        let items: Vec<BucketItem> = buckets
            .values()
            .map(|info| BucketItem {
                name: info.name.clone(),
                creation_date: info.created_at.to_rfc3339(),
            })
            .collect();

        GatewayHttpResponse::ok_xml(ListAllMyBucketsResult::new(items).to_xml())
    }

    fn handle_head_bucket(&self, bucket: &str) -> GatewayHttpResponse {
        if self.buckets.read().unwrap().contains_key(bucket) {
            let mut headers = HashMap::new();
            headers.insert("x-amz-bucket-region".to_string(), "us-east-1".to_string());
            headers.insert("Content-Length".to_string(), "0".to_string());
            GatewayHttpResponse {
                status: 200,
                headers,
                body: Bytes::new(),
            }
        } else {
            GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            )
        }
    }

    fn handle_get_bucket_location(&self, bucket: &str) -> GatewayHttpResponse {
        if !self.buckets.read().unwrap().contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            );
        }
        GatewayHttpResponse::ok_xml(LocationConstraint::new("us-east-1").to_xml())
    }

    fn handle_get_bucket_versioning(&self, bucket: &str) -> GatewayHttpResponse {
        let buckets = self.buckets.read().unwrap();
        let info = match buckets.get(bucket) {
            Some(i) => i,
            None => {
                return GatewayHttpResponse::error(
                    S3ErrorCode::NoSuchBucket,
                    "O bucket especificado não existe",
                    Some(bucket.to_string()),
                );
            }
        };

        let status_str = match info.versioning {
            BucketVersioningStatus::Enabled => Some("Enabled".to_string()),
            BucketVersioningStatus::Suspended => Some("Suspended".to_string()),
            BucketVersioningStatus::Off => None,
        };

        GatewayHttpResponse::ok_xml(VersioningConfiguration::new(status_str).to_xml())
    }

    fn handle_put_bucket_versioning(&self, bucket: &str, body: &[u8]) -> GatewayHttpResponse {
        let mut buckets = self.buckets.write().unwrap();
        let info = match buckets.get_mut(bucket) {
            Some(i) => i,
            None => {
                return GatewayHttpResponse::error(
                    S3ErrorCode::NoSuchBucket,
                    "O bucket especificado não existe",
                    Some(bucket.to_string()),
                );
            }
        };

        let body_str = String::from_utf8_lossy(body);
        if body_str.contains("Enabled") {
            info.versioning = BucketVersioningStatus::Enabled;
        } else if body_str.contains("Suspended") {
            info.versioning = BucketVersioningStatus::Suspended;
        }

        drop(buckets);
        self.persist_buckets();
        GatewayHttpResponse::ok_empty()
    }

    fn handle_get_bucket_acl(&self, bucket: &str) -> GatewayHttpResponse {
        if !self.buckets.read().unwrap().contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            );
        }
        GatewayHttpResponse::ok_xml(AccessControlPolicy::default_owner("z3sadmin").to_xml())
    }

    fn handle_put_bucket_acl(&self, bucket: &str) -> GatewayHttpResponse {
        if !self.buckets.read().unwrap().contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            );
        }
        GatewayHttpResponse::ok_empty()
    }

    fn handle_get_bucket_policy(&self, bucket: &str) -> GatewayHttpResponse {
        if !self.buckets.read().unwrap().contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            );
        }
        GatewayHttpResponse::error(
            S3ErrorCode::NoSuchBucketPolicy,
            "The bucket policy does not exist",
            Some(bucket.to_string()),
        )
    }

    fn handle_get_bucket_cors(&self, bucket: &str) -> GatewayHttpResponse {
        if !self.buckets.read().unwrap().contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            );
        }
        GatewayHttpResponse::error(
            S3ErrorCode::NoSuchCORSConfiguration,
            "The CORS configuration does not exist",
            Some(bucket.to_string()),
        )
    }

    fn handle_get_bucket_lifecycle(&self, bucket: &str) -> GatewayHttpResponse {
        if !self.buckets.read().unwrap().contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            );
        }
        GatewayHttpResponse::error(
            S3ErrorCode::NoSuchLifecycleConfiguration,
            "The lifecycle configuration does not exist",
            Some(bucket.to_string()),
        )
    }

    fn handle_get_bucket_tagging(&self, bucket: &str) -> GatewayHttpResponse {
        if !self.buckets.read().unwrap().contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            );
        }
        GatewayHttpResponse::error(
            S3ErrorCode::NoSuchTagSet,
            "There is no tag set associated with the bucket",
            Some(bucket.to_string()),
        )
    }

    fn handle_get_bucket_encryption(&self, bucket: &str) -> GatewayHttpResponse {
        if !self.buckets.read().unwrap().contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            );
        }
        GatewayHttpResponse::error(
            S3ErrorCode::ServerSideEncryptionConfigurationNotFoundError,
            "The server side encryption configuration was not found",
            Some(bucket.to_string()),
        )
    }

    fn handle_get_public_access_block(&self, bucket: &str) -> GatewayHttpResponse {
        if !self.buckets.read().unwrap().contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            );
        }
        GatewayHttpResponse::error(
            S3ErrorCode::NoSuchPublicAccessBlockConfiguration,
            "The public access block configuration was not found",
            Some(bucket.to_string()),
        )
    }

    fn handle_list_multipart_uploads(&self, bucket: &str) -> GatewayHttpResponse {
        if !self.buckets.read().unwrap().contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            );
        }
        GatewayHttpResponse::ok_xml(ListMultipartUploadsResult::new(bucket).to_xml())
    }

    fn handle_create_bucket(&self, bucket: &str) -> GatewayHttpResponse {
        if BucketName::new(bucket).is_err() {
            return GatewayHttpResponse::error(
                S3ErrorCode::InvalidBucketName,
                "O nome do bucket especificado é inválido",
                Some(bucket.to_string()),
            );
        }

        let mut map = self.buckets.write().unwrap();
        if map.contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::BucketAlreadyOwnedByYou,
                "Seu bucket já existe",
                Some(bucket.to_string()),
            );
        }

        map.insert(bucket.to_string(), BucketInfo {
            name: bucket.to_string(),
            created_at: chrono::Utc::now(),
            versioning: BucketVersioningStatus::Off,
        });
        drop(map);
        self.persist_buckets();

        let mut resp = GatewayHttpResponse::ok_xml(String::new());
        resp.headers.insert("Location".to_string(), format!("/{}", bucket));
        resp.headers.insert("Content-Length".to_string(), "0".to_string());
        resp
    }

    fn handle_delete_bucket(&self, bucket: &str) -> GatewayHttpResponse {
        let mut map = self.buckets.write().unwrap();
        if !map.contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            );
        }

        let objects = self.objects.read().unwrap();
        let bucket_prefix = format!("{}/", bucket);
        let has_objects = objects.iter().any(|(k, v)| k.starts_with(&bucket_prefix) && !v.is_empty());
        if has_objects {
            return GatewayHttpResponse::error(
                S3ErrorCode::BucketNotEmpty,
                "O bucket a ser deletado não está vazio",
                Some(bucket.to_string()),
            );
        }
        drop(objects);

        map.remove(bucket);
        drop(map);
        self.persist_buckets();

        let mut headers = HashMap::new();
        headers.insert("Content-Length".to_string(), "0".to_string());
        GatewayHttpResponse {
            status: 204,
            headers,
            body: Bytes::new(),
        }
    }

    fn handle_put_object(
        &self,
        bucket: &str,
        key: &str,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> GatewayHttpResponse {
        let bucket_name = match BucketName::new(bucket) {
            Ok(b) => b,
            Err(_) => {
                return GatewayHttpResponse::error(
                    S3ErrorCode::InvalidBucketName,
                    "Nome de bucket inválido",
                    Some(bucket.to_string()),
                )
            }
        };

        let object_key = match ObjectKey::new(key) {
            Ok(k) => k,
            Err(_) => {
                return GatewayHttpResponse::error(
                    S3ErrorCode::InvalidArgument,
                    "Chave de objeto inválida",
                    Some(key.to_string()),
                )
            }
        };

        let versioning_status = {
            let buckets = self.buckets.read().unwrap();
            match buckets.get(bucket) {
                Some(b) => b.versioning,
                None => {
                    return GatewayHttpResponse::error(
                        S3ErrorCode::NoSuchBucket,
                        "O bucket especificado não existe",
                        Some(bucket.to_string()),
                    );
                }
            }
        };

        let encoded = match self.erasure.encode(body) {
            Ok(enc) => enc,
            Err(e) => {
                return GatewayHttpResponse::error(
                    S3ErrorCode::InternalError,
                    format!("Erro de codificação: {}", e),
                    None,
                )
            }
        };

        let total_shards = self.erasure.total_shards() as u32;
        let mut shard_pointers = Vec::new();

        for (i, shard_bytes) in encoded.all_shards().into_iter().enumerate() {
            let shard_id = Uuid::new_v4();
            let is_parity = i >= self.erasure.data_shards();

            let location = match self
                .storage
                .write_shard(shard_id, i as u32, total_shards, &shard_bytes)
            {
                Ok(loc) => loc,
                Err(e) => {
                    return GatewayHttpResponse::error(
                        S3ErrorCode::InternalError,
                        format!("Erro de I/O em disco: {}", e),
                        None,
                    )
                }
            };

            shard_pointers.push(ShardPointer {
                shard_id: ShardId(shard_id),
                shard_index: i as u32,
                is_parity,
                node_id: self.node_id,
                extent_id: location.extent_id,
                offset_in_extent: location.offset_in_extent,
                length: location.payload_length,
                blake3_checksum: location.checksum_blake3,
            });
        }

        let etag = ETag::from_hex(calculate_md5_etag(body));
        let content_type = headers
            .get("content-type")
            .cloned()
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let mut user_metadata = HashMap::new();
        for (k, v) in headers {
            if k.starts_with("x-amz-meta-") {
                user_metadata.insert(k.clone(), v.clone());
            }
        }

        let (version_id_str, version_id) = match versioning_status {
            BucketVersioningStatus::Enabled => {
                let vid = Uuid::now_v7().to_string();
                (vid.clone(), VersionId::new(vid))
            }
            _ => ("null".to_string(), VersionId::new("null")),
        };

        let metadata = ObjectMetadata {
            bucket: bucket_name,
            key: object_key,
            version_id,
            size: body.len() as u64,
            etag: etag.clone(),
            content_type,
            storage_class: StorageClass::Standard,
            created_at: chrono::Utc::now(),
            user_metadata,
            merkle_root: blake3_hash(body),
            is_delete_marker: false,
            is_latest: true,
        };

        let manifest = ObjectManifest::new(
            metadata,
            self.erasure.data_shards() as u32,
            self.erasure.parity_shards() as u32,
            shard_pointers,
        );

        let manifest_key = format!("{}/{}", bucket, key);
        {
            let mut objects = self.objects.write().unwrap();
            let list = objects.entry(manifest_key).or_default();
            match versioning_status {
                BucketVersioningStatus::Enabled => {
                    for item in list.iter_mut() {
                        item.metadata.is_latest = false;
                    }
                    list.insert(0, manifest);
                }
                BucketVersioningStatus::Suspended | BucketVersioningStatus::Off => {
                    if let Some(pos) = list.iter().position(|v| v.metadata.version_id.as_str() == "null") {
                        let old = list.remove(pos);
                        for shard in &old.shards {
                            let _ = self.storage.delete_shard(&shard.shard_id.0);
                        }
                    }
                    for item in list.iter_mut() {
                        item.metadata.is_latest = false;
                    }
                    list.insert(0, manifest);
                }
            }
        }
        self.persist_objects();

        let mut resp_headers = HashMap::new();
        resp_headers.insert("ETag".to_string(), etag.as_str().to_string());
        resp_headers.insert("Content-Length".to_string(), "0".to_string());
        if versioning_status == BucketVersioningStatus::Enabled || version_id_str != "null" {
            resp_headers.insert("x-amz-version-id".to_string(), version_id_str);
        }
        GatewayHttpResponse {
            status: 200,
            headers: resp_headers,
            body: Bytes::new(),
        }
    }

    pub fn clean_xml_text(s: &str) -> String {
        let s = s.trim();
        let s = if let Some(inside) = s.strip_prefix("<![CDATA[").and_then(|t| t.strip_suffix("]]>")) {
            inside
        } else {
            s
        };
        s.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
    }

    /// Implementação de CopyObject (usado para renomear/mover arquivos no S3)
    fn handle_copy_object(
        &self,
        dest_bucket: &str,
        dest_key: &str,
        src_bucket: &str,
        src_key: &str,
        headers: &HashMap<String, String>,
    ) -> GatewayHttpResponse {
        let src_manifest_key = format!("{}/{}", src_bucket, src_key);
        let src_manifest = {
            let map = self.objects.read().unwrap();
            let list = map.get(&src_manifest_key)
                .or_else(|| {
                    if !src_key.ends_with('/') {
                        map.get(&format!("{}/{}/", src_bucket, src_key))
                    } else {
                        map.get(&format!("{}/{}", src_bucket, src_key.trim_end_matches('/')))
                    }
                });

            list.and_then(|versions| versions.iter().find(|v| v.metadata.is_latest).or_else(|| versions.first())).cloned()
        };
        let src_manifest = match src_manifest {
            Some(m) => {
                if m.metadata.is_delete_marker {
                    return GatewayHttpResponse::error(
                        S3ErrorCode::NoSuchKey,
                        "O objeto de origem é um Delete Marker",
                        Some(src_key.to_string()),
                    );
                }
                m
            }
            None => {
                return GatewayHttpResponse::error(
                    S3ErrorCode::NoSuchKey,
                    "O objeto de origem especificado em x-amz-copy-source não existe",
                    Some(src_key.to_string()),
                )
            }
        };

        // Reconstrói o payload de origem
        let mut shards_options: Vec<Option<Vec<u8>>> = Vec::new();
        for shard in &src_manifest.shards {
            match self.storage.read_shard(&shard.shard_id.0) {
                Ok(data) => shards_options.push(Some(data)),
                Err(_) => shards_options.push(None),
            }
        }

        let payload = match self
            .erasure
            .reconstruct(&mut shards_options, src_manifest.metadata.size as usize)
        {
            Ok(p) => p,
            Err(e) => {
                return GatewayHttpResponse::error(
                    S3ErrorCode::InternalError,
                    format!("Erro ao ler dados de origem para cópia: {}", e),
                    None,
                )
            }
        };

        // Mescla metadados de origem com novos metadados se fornecidos
        let mut final_headers = headers.clone();
        for (k, v) in &src_manifest.metadata.user_metadata {
            final_headers.entry(k.clone()).or_insert_with(|| v.clone());
        }

        // Grava no destino
        let put_resp = self.handle_put_object(dest_bucket, dest_key, &final_headers, &payload);
        if put_resp.status != 200 {
            return put_resp;
        }

        let now = chrono::Utc::now().to_rfc3339();
        let etag = src_manifest.metadata.etag.as_str().to_string();

        GatewayHttpResponse::ok_xml(CopyObjectResult::new(now, etag).to_xml())
    }

    fn handle_get_object(
        &self,
        bucket: &str,
        key: &str,
        version_id: Option<&str>,
        range: Option<ByteRange>,
        headers: &HashMap<String, String>,
    ) -> GatewayHttpResponse {
        let manifest_key = format!("{}/{}", bucket, key);
        let manifest = {
            let map = self.objects.read().unwrap();
            let list = map.get(&manifest_key)
                .or_else(|| {
                    if !key.ends_with('/') {
                        map.get(&format!("{}/{}/", bucket, key))
                    } else {
                        map.get(&format!("{}/{}", bucket, key.trim_end_matches('/')))
                    }
                })
                .or_else(|| {
                    map.get(&format!("{}/{}_$folder$", bucket, key.trim_end_matches('/')))
                });

            match list {
                Some(versions) => {
                    if let Some(vid) = version_id {
                        versions.iter().find(|v| v.metadata.version_id.as_str() == vid).cloned()
                    } else {
                        versions.iter().find(|v| v.metadata.is_latest).or_else(|| versions.first()).cloned()
                    }
                }
                None => None,
            }
        };

        let manifest = match manifest {
            Some(m) => {
                if m.metadata.is_delete_marker {
                    let mut resp_headers = HashMap::new();
                    resp_headers.insert("x-amz-delete-marker".to_string(), "true".to_string());
                    resp_headers.insert("x-amz-version-id".to_string(), m.metadata.version_id.as_str().to_string());
                    let mut resp = GatewayHttpResponse::error(
                        S3ErrorCode::NoSuchKey,
                        "A chave especificada é um Delete Marker",
                        Some(key.to_string()),
                    );
                    resp.headers.extend(resp_headers);
                    return resp;
                }
                m
            }
            None => {
                return GatewayHttpResponse::error(
                    S3ErrorCode::NoSuchKey,
                    "A chave especificada não existe",
                    Some(key.to_string()),
                )
            }
        };

        let etag_str = manifest.metadata.etag.as_str().trim_matches('"');
        if let Some(if_match) = headers.get("if-match") {
            let if_match = if_match.trim().trim_matches('"');
            if if_match != "*" && if_match != etag_str {
                return GatewayHttpResponse::error(
                    S3ErrorCode::PreconditionFailed,
                    "No momento o ETag do objeto difere do especificado em If-Match",
                    Some(key.to_string()),
                );
            }
        }

        if let Some(if_none_match) = headers.get("if-none-match") {
            let if_none_match = if_none_match.trim().trim_matches('"');
            if if_none_match == "*" || if_none_match == etag_str {
                let mut resp = GatewayHttpResponse {
                    status: 304,
                    headers: HashMap::new(),
                    body: Bytes::new(),
                };
                resp.headers
                    .insert("ETag".to_string(), manifest.metadata.etag.as_str().to_string());
                resp.headers.insert("Content-Length".to_string(), "0".to_string());
                return resp;
            }
        }

        let full_payload = if !manifest.parts.is_empty() {
            let mut payload = Vec::with_capacity(manifest.metadata.size as usize);
            for part in &manifest.parts {
                let mut shards_options: Vec<Option<Vec<u8>>> = Vec::new();
                for shard in &part.shards {
                    match self.storage.read_shard(&shard.shard_id.0) {
                        Ok(data) => shards_options.push(Some(data)),
                        Err(_) => shards_options.push(None),
                    }
                }
                match self.erasure.reconstruct(&mut shards_options, part.size as usize) {
                    Ok(p_data) => payload.extend(p_data),
                    Err(e) => {
                        return GatewayHttpResponse::error(
                            S3ErrorCode::InternalError,
                            format!("Erro de reconstrução de parte: {}", e),
                            None,
                        );
                    }
                }
            }
            payload
        } else {
            let mut shards_options: Vec<Option<Vec<u8>>> = Vec::new();
            for shard in &manifest.shards {
                match self.storage.read_shard(&shard.shard_id.0) {
                    Ok(data) => shards_options.push(Some(data)),
                    Err(_) => shards_options.push(None),
                }
            }
            match self.erasure.reconstruct(&mut shards_options, manifest.metadata.size as usize) {
                Ok(payload) => payload,
                Err(e) => {
                    return GatewayHttpResponse::error(
                        S3ErrorCode::InternalError,
                        format!("Erro de reconstrução: {}", e),
                        None,
                    );
                }
            }
        };

        let mut resp = if let Some(r) = range {
            let total = full_payload.len() as u64;
            let start = r.start;
            let end = r.end.unwrap_or(total.saturating_sub(1)).min(total.saturating_sub(1));

            if start >= total || start > end {
                return GatewayHttpResponse::error(
                    S3ErrorCode::InvalidRange,
                    "O intervalo de bytes solicitado é inválido",
                    None,
                );
            }

            let slice = full_payload[start as usize..=end as usize].to_vec();
            GatewayHttpResponse::partial_content(
                slice,
                start,
                end,
                total,
                &manifest.metadata.content_type,
            )
        } else {
            GatewayHttpResponse::ok_bytes(
                full_payload,
                &manifest.metadata.content_type,
                &manifest.metadata.etag,
            )
        };

        for (k, v) in &manifest.metadata.user_metadata {
            resp.headers.insert(k.clone(), v.clone());
        }
        resp.headers.insert(
            "Last-Modified".to_string(),
            manifest.metadata.created_at.format("%a, %d %b %Y %H:%M:%S GMT").to_string(),
        );
        if manifest.metadata.version_id.as_str() != "null" {
            resp.headers.insert(
                "x-amz-version-id".to_string(),
                manifest.metadata.version_id.as_str().to_string(),
            );
        }

        resp
    }

    fn handle_get_object_acl(&self, bucket: &str, key: &str) -> GatewayHttpResponse {
        let manifest_key = format!("{}/{}", bucket, key);
        let exists = self.objects.read().unwrap().contains_key(&manifest_key);
        if !exists {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchKey,
                "A chave especificada não existe",
                Some(key.to_string()),
            );
        }
        GatewayHttpResponse::ok_xml(AccessControlPolicy::default_owner("z3sadmin").to_xml())
    }

    fn handle_put_object_acl(&self, bucket: &str, key: &str) -> GatewayHttpResponse {
        let manifest_key = format!("{}/{}", bucket, key);
        let exists = self.objects.read().unwrap().contains_key(&manifest_key);
        if !exists {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchKey,
                "A chave especificada não existe",
                Some(key.to_string()),
            );
        }
        GatewayHttpResponse::ok_empty()
    }

    fn handle_head_object(&self, bucket: &str, key: &str, version_id: Option<&str>) -> GatewayHttpResponse {
        let manifest_key = format!("{}/{}", bucket, key);
        let manifest = {
            let map = self.objects.read().unwrap();
            let list = map.get(&manifest_key)
                .or_else(|| {
                    if !key.ends_with('/') {
                        map.get(&format!("{}/{}/", bucket, key))
                    } else {
                        map.get(&format!("{}/{}", bucket, key.trim_end_matches('/')))
                    }
                })
                .or_else(|| {
                    map.get(&format!("{}/{}_$folder$", bucket, key.trim_end_matches('/')))
                });

            match list {
                Some(versions) => {
                    if let Some(vid) = version_id {
                        versions.iter().find(|v| v.metadata.version_id.as_str() == vid).cloned()
                    } else {
                        versions.iter().find(|v| v.metadata.is_latest).or_else(|| versions.first()).cloned()
                    }
                }
                None => None,
            }
        };

        let manifest = match manifest {
            Some(m) => {
                if m.metadata.is_delete_marker {
                    let mut resp_headers = HashMap::new();
                    resp_headers.insert("x-amz-delete-marker".to_string(), "true".to_string());
                    resp_headers.insert("x-amz-version-id".to_string(), m.metadata.version_id.as_str().to_string());
                    let mut resp = GatewayHttpResponse::error(
                        S3ErrorCode::NoSuchKey,
                        "A chave especificada é um Delete Marker",
                        Some(key.to_string()),
                    );
                    resp.headers.extend(resp_headers);
                    return resp;
                }
                m
            }
            None => {
                return GatewayHttpResponse::error(
                    S3ErrorCode::NoSuchKey,
                    "A chave especificada não existe",
                    Some(key.to_string()),
                )
            }
        };

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), manifest.metadata.content_type);
        headers.insert("Content-Length".to_string(), manifest.metadata.size.to_string());
        headers.insert("ETag".to_string(), manifest.metadata.etag.as_str().to_string());
        headers.insert("Accept-Ranges".to_string(), "bytes".to_string());
        headers.insert(
            "Last-Modified".to_string(),
            manifest.metadata.created_at.format("%a, %d %b %Y %H:%M:%S GMT").to_string(),
        );
        if manifest.metadata.version_id.as_str() != "null" {
            headers.insert(
                "x-amz-version-id".to_string(),
                manifest.metadata.version_id.as_str().to_string(),
            );
        }

        for (k, v) in &manifest.metadata.user_metadata {
            headers.insert(k.clone(), v.clone());
        }

        GatewayHttpResponse {
            status: 200,
            headers,
            body: Bytes::new(),
        }
    }

    fn handle_delete_object(&self, bucket: &str, key: &str, version_id: Option<&str>) -> GatewayHttpResponse {
        let versioning_status = {
            let buckets = self.buckets.read().unwrap();
            match buckets.get(bucket) {
                Some(b) => b.versioning,
                None => {
                    return GatewayHttpResponse::error(
                        S3ErrorCode::NoSuchBucket,
                        "O bucket especificado não existe",
                        Some(bucket.to_string()),
                    );
                }
            }
        };

        let manifest_key = format!("{}/{}", bucket, key);
        let mut map = self.objects.write().unwrap();

        if let Some(vid) = version_id {
            // Exclui versão específica permanentemente
            if let Some(versions) = map.get_mut(&manifest_key) {
                if let Some(pos) = versions.iter().position(|v| v.metadata.version_id.as_str() == vid) {
                    let removed = versions.remove(pos);
                    if !removed.metadata.is_delete_marker {
                        for shard in &removed.shards {
                            let _ = self.storage.delete_shard(&shard.shard_id.0);
                        }
                    }
                    if removed.metadata.is_latest && !versions.is_empty() {
                        versions[0].metadata.is_latest = true;
                    }
                    let is_del_marker = removed.metadata.is_delete_marker;
                    if versions.is_empty() {
                        map.remove(&manifest_key);
                    }
                    drop(map);
                    self.persist_objects();

                    let mut headers = HashMap::new();
                    headers.insert("Content-Length".to_string(), "0".to_string());
                    headers.insert("x-amz-version-id".to_string(), vid.to_string());
                    if is_del_marker {
                        headers.insert("x-amz-delete-marker".to_string(), "true".to_string());
                    }
                    return GatewayHttpResponse {
                        status: 204,
                        headers,
                        body: Bytes::new(),
                    };
                }
            }
            drop(map);
            let mut headers = HashMap::new();
            headers.insert("Content-Length".to_string(), "0".to_string());
            return GatewayHttpResponse {
                status: 204,
                headers,
                body: Bytes::new(),
            };
        }

        // Sem version_id:
        if versioning_status == BucketVersioningStatus::Enabled || versioning_status == BucketVersioningStatus::Suspended {
            // Cria um Delete Marker
            let marker_vid = Uuid::now_v7().to_string();
            let marker_meta = ObjectMetadata::new_delete_marker(
                BucketName::new(bucket).unwrap_or_else(|_| BucketName::new("default").unwrap()),
                ObjectKey::new(key).unwrap_or_else(|_| ObjectKey::new("default").unwrap()),
                VersionId::new(&marker_vid),
            );
            let marker_manifest = ObjectManifest::new(
                marker_meta,
                self.erasure.data_shards() as u32,
                self.erasure.parity_shards() as u32,
                Vec::new(),
            );

            let list = map.entry(manifest_key).or_default();
            for item in list.iter_mut() {
                item.metadata.is_latest = false;
            }
            list.insert(0, marker_manifest);
            drop(map);
            self.persist_objects();

            let mut headers = HashMap::new();
            headers.insert("Content-Length".to_string(), "0".to_string());
            headers.insert("x-amz-delete-marker".to_string(), "true".to_string());
            headers.insert("x-amz-version-id".to_string(), marker_vid);
            return GatewayHttpResponse {
                status: 204,
                headers,
                body: Bytes::new(),
            };
        } else {
            // Versionamento Off: remove todas as versões
            let removed = map.remove(&manifest_key)
                .or_else(|| {
                    if !key.ends_with('/') {
                        map.remove(&format!("{}/{}/", bucket, key))
                    } else {
                        map.remove(&format!("{}/{}", bucket, key.trim_end_matches('/')))
                    }
                })
                .or_else(|| {
                    map.remove(&format!("{}/{}_$folder$", bucket, key.trim_end_matches('/')))
                });

            if let Some(versions) = removed {
                for v in versions {
                    if !v.metadata.is_delete_marker {
                        for shard in &v.shards {
                            let _ = self.storage.delete_shard(&shard.shard_id.0);
                        }
                    }
                }
                drop(map);
                self.persist_objects();
            }
            let mut headers = HashMap::new();
            headers.insert("Content-Length".to_string(), "0".to_string());
            GatewayHttpResponse {
                status: 204,
                headers,
                body: Bytes::new(),
            }
        }
    }

    /// Implementação de DeleteObjects (Multi-Object Delete via POST /bucket?delete)
    fn handle_delete_objects(&self, bucket: &str, body: &[u8]) -> GatewayHttpResponse {
        let body_str = match std::str::from_utf8(body) {
            Ok(s) => s,
            Err(_) => {
                return GatewayHttpResponse::error(
                    S3ErrorCode::MalformedXML,
                    "Payload XML de exclusão inválido",
                    None,
                )
            }
        };

        let versioning_status = {
            let buckets = self.buckets.read().unwrap();
            match buckets.get(bucket) {
                Some(b) => b.versioning,
                None => {
                    return GatewayHttpResponse::error(
                        S3ErrorCode::NoSuchBucket,
                        "O bucket especificado não existe",
                        Some(bucket.to_string()),
                    );
                }
            }
        };

        let mut deleted_items = Vec::new();
        let mut map = self.objects.write().unwrap();

        // Extrai objetos <Object><Key>...</Key><VersionId>...</VersionId></Object> ou <Key>...</Key>
        let mut remaining = body_str;
        while let Some(start_idx) = remaining.find("<Key>") {
            let after_start = &remaining[start_idx + 5..];
            if let Some(end_idx) = after_start.find("</Key>") {
                let raw_key = &after_start[..end_idx];
                let key = Self::clean_xml_text(raw_key);
                let manifest_key = format!("{}/{}", bucket, key);

                // Checa se há VersionId correspondente
                let mut version_id = None;
                let after_key = &after_start[end_idx + 6..];
                if let Some(v_start) = after_key.find("<VersionId>") {
                    let next_obj_or_key = after_key.find("<Key>").unwrap_or(usize::MAX);
                    if v_start < next_obj_or_key {
                        let after_v = &after_key[v_start + 11..];
                        if let Some(v_end) = after_v.find("</VersionId>") {
                            version_id = Some(Self::clean_xml_text(&after_v[..v_end]));
                        }
                    }
                }

                if let Some(ref vid) = version_id {
                    let mut was_del_marker = false;
                    if let Some(versions) = map.get_mut(&manifest_key) {
                        if let Some(pos) = versions.iter().position(|v| v.metadata.version_id.as_str() == vid) {
                            let removed = versions.remove(pos);
                            if !removed.metadata.is_delete_marker {
                                for shard in &removed.shards {
                                    let _ = self.storage.delete_shard(&shard.shard_id.0);
                                }
                            }
                            was_del_marker = removed.metadata.is_delete_marker;
                            if removed.metadata.is_latest && !versions.is_empty() {
                                versions[0].metadata.is_latest = true;
                            }
                            if versions.is_empty() {
                                map.remove(&manifest_key);
                            }
                        }
                    }
                    deleted_items.push(DeletedItem {
                        key: key.to_string(),
                        version_id: Some(vid.clone()),
                        delete_marker: if was_del_marker { Some(true) } else { None },
                        delete_marker_version_id: if was_del_marker { Some(vid.clone()) } else { None },
                    });
                } else if versioning_status == BucketVersioningStatus::Enabled || versioning_status == BucketVersioningStatus::Suspended {
                    let marker_vid = Uuid::now_v7().to_string();
                    let marker_meta = ObjectMetadata::new_delete_marker(
                        BucketName::new(bucket).unwrap_or_else(|_| BucketName::new("default").unwrap()),
                        ObjectKey::new(&key).unwrap_or_else(|_| ObjectKey::new("default").unwrap()),
                        VersionId::new(&marker_vid),
                    );
                    let marker_manifest = ObjectManifest::new(
                        marker_meta,
                        self.erasure.data_shards() as u32,
                        self.erasure.parity_shards() as u32,
                        Vec::new(),
                    );

                    let list = map.entry(manifest_key).or_default();
                    for item in list.iter_mut() {
                        item.metadata.is_latest = false;
                    }
                    list.insert(0, marker_manifest);

                    deleted_items.push(DeletedItem {
                        key: key.to_string(),
                        version_id: None,
                        delete_marker: Some(true),
                        delete_marker_version_id: Some(marker_vid),
                    });
                } else {
                    let removed = map.remove(&manifest_key)
                        .or_else(|| {
                            if !key.ends_with('/') {
                                map.remove(&format!("{}/{}/", bucket, key))
                            } else {
                                map.remove(&format!("{}/{}", bucket, key.trim_end_matches('/')))
                            }
                        })
                        .or_else(|| {
                            map.remove(&format!("{}/{}_$folder$", bucket, key.trim_end_matches('/')))
                        });

                    if let Some(versions) = removed {
                        for v in versions {
                            if !v.metadata.is_delete_marker {
                                for shard in &v.shards {
                                    let _ = self.storage.delete_shard(&shard.shard_id.0);
                                }
                            }
                        }
                    }
                    deleted_items.push(DeletedItem {
                        key: key.to_string(),
                        version_id: None,
                        delete_marker: None,
                        delete_marker_version_id: None,
                    });
                }

                remaining = &after_start[end_idx + 6..];
            } else {
                break;
            }
        }
        drop(map);
        self.persist_objects();

        GatewayHttpResponse::ok_xml(DeleteResult::new(deleted_items).to_xml())
    }

    fn handle_list_objects(
        &self,
        bucket: &str,
        prefix: &str,
        delimiter: Option<String>,
        max_keys: usize,
    ) -> GatewayHttpResponse {
        let buckets = self.buckets.read().unwrap();
        if !buckets.contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            );
        }
        drop(buckets);

        let map = self.objects.read().unwrap();
        let bucket_prefix = format!("{}/{}", bucket, prefix);

        let mut contents = Vec::new();
        let mut common_prefixes_set = HashSet::new();

        for (k, versions) in map.iter() {
            if k.starts_with(&bucket_prefix) {
                // Obter a versão mais recente
                let latest = match versions.iter().find(|v| v.metadata.is_latest).or_else(|| versions.first()) {
                    Some(m) => m,
                    None => continue,
                };

                // Delete Markers são ocultos em ListObjectsV2
                if latest.metadata.is_delete_marker {
                    continue;
                }

                let relative_key = &k[bucket.len() + 1..];

                // Identifica se este objeto é uma pasta virtual
                let is_folder = relative_key.ends_with('/')
                    || relative_key.ends_with("_$folder$")
                    || latest.metadata.content_type.contains("directory");

                if let Some(ref delim) = delimiter {
                    if !delim.is_empty() {
                        let sub_key = &relative_key[prefix.len()..];
                        if let Some(idx) = sub_key.find(delim) {
                            let folder_prefix = format!("{}{}", prefix, &sub_key[..idx + delim.len()]);
                            common_prefixes_set.insert(folder_prefix);
                            continue;
                        } else if is_folder {
                            let folder_name = relative_key
                                .strip_suffix("_$folder$")
                                .unwrap_or(relative_key)
                                .trim_end_matches('/');
                            let folder_prefix = format!("{}/", folder_name);
                            if folder_prefix != prefix {
                                common_prefixes_set.insert(folder_prefix);
                                continue;
                            }
                        }
                    }
                }

                if delimiter.is_some() {
                    // Quando há delimiter (navegação em diretório), não lista o próprio marcador de diretório como item de arquivo
                    if is_folder && (relative_key == prefix || format!("{}/", relative_key.trim_end_matches('/')) == prefix) {
                        continue;
                    }
                    if is_folder {
                        continue;
                    }
                }

                contents.push(ObjectItem {
                    key: latest.metadata.key.as_str().to_string(),
                    last_modified: latest.metadata.created_at.to_rfc3339(),
                    etag: latest.metadata.etag.as_str().to_string(),
                    size: latest.metadata.size,
                    storage_class: "STANDARD".to_string(),
                });
            }
        }

        contents.sort_by(|a, b| a.key.cmp(&b.key));
        contents.truncate(max_keys);

        let mut common_prefixes: Vec<CommonPrefixItem> = common_prefixes_set
            .into_iter()
            .map(|p| CommonPrefixItem { prefix: p })
            .collect();
        common_prefixes.sort_by(|a, b| a.prefix.cmp(&b.prefix));

        GatewayHttpResponse::ok_xml(
            ListBucketResult::new(
                bucket.to_string(),
                prefix.to_string(),
                delimiter,
                max_keys,
                contents,
                common_prefixes,
            )
            .to_xml(),
        )
    }

    fn handle_list_object_versions(
        &self,
        bucket: &str,
        prefix: &str,
        delimiter: Option<String>,
        _key_marker: Option<String>,
        _version_id_marker: Option<String>,
        max_keys: usize,
    ) -> GatewayHttpResponse {
        if !self.buckets.read().unwrap().contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            );
        }

        let map = self.objects.read().unwrap();
        let bucket_prefix = format!("{}/", bucket);

        let mut versions_list = Vec::new();
        let mut delete_markers_list = Vec::new();
        let mut common_prefixes_set = std::collections::BTreeSet::new();

        for (k, versions) in map.iter() {
            if k.starts_with(&bucket_prefix) {
                let relative_key = &k[bucket.len() + 1..];

                if !prefix.is_empty() && !relative_key.starts_with(prefix) {
                    continue;
                }

                if let Some(ref delim) = delimiter {
                    if !delim.is_empty() {
                        let sub_key = &relative_key[prefix.len()..];
                        if let Some(idx) = sub_key.find(delim) {
                            let folder_prefix = format!("{}{}", prefix, &sub_key[..idx + delim.len()]);
                            common_prefixes_set.insert(folder_prefix);
                            continue;
                        }
                    }
                }

                for v in versions {
                    let owner = crate::xml::AclOwner {
                        id: "z3s-admin".to_string(),
                        display_name: "z3s-admin".to_string(),
                    };

                    if v.metadata.is_delete_marker {
                        delete_markers_list.push(DeleteMarkerItem {
                            key: v.metadata.key.as_str().to_string(),
                            version_id: v.metadata.version_id.as_str().to_string(),
                            is_latest: v.metadata.is_latest,
                            last_modified: v.metadata.created_at.to_rfc3339(),
                            owner,
                        });
                    } else {
                        versions_list.push(VersionItem {
                            key: v.metadata.key.as_str().to_string(),
                            version_id: v.metadata.version_id.as_str().to_string(),
                            is_latest: v.metadata.is_latest,
                            last_modified: v.metadata.created_at.to_rfc3339(),
                            etag: v.metadata.etag.as_str().to_string(),
                            size: v.metadata.size,
                            owner,
                            storage_class: "STANDARD".to_string(),
                        });
                    }
                }
            }
        }

        versions_list.sort_by(|a, b| a.key.cmp(&b.key).then_with(|| b.last_modified.cmp(&a.last_modified)));
        versions_list.truncate(max_keys);

        delete_markers_list.sort_by(|a, b| a.key.cmp(&b.key).then_with(|| b.last_modified.cmp(&a.last_modified)));
        delete_markers_list.truncate(max_keys);

        let common_prefixes: Vec<CommonPrefixItem> = common_prefixes_set
            .into_iter()
            .map(|p| CommonPrefixItem { prefix: p })
            .collect();

        GatewayHttpResponse::ok_xml(
            ListVersionsResult::new(
                bucket.to_string(),
                prefix.to_string(),
                delimiter,
                versions_list,
                delete_markers_list,
                common_prefixes,
            )
            .to_xml(),
        )
    }

    fn handle_initiate_multipart(&self, bucket: &str, key: &str) -> GatewayHttpResponse {
        if !self.buckets.read().unwrap().contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            );
        }

        let upload_id = Uuid::new_v4().to_string();
        self.multiparts.write().unwrap().insert(
            upload_id.clone(),
            ActiveMultipartUpload {
                parts: HashMap::new(),
            },
        );

        GatewayHttpResponse::ok_xml(
            InitiateMultipartUploadResult::new(bucket, key, upload_id).to_xml(),
        )
    }

    fn handle_upload_part(
        &self,
        bucket: &str,
        _key: &str,
        upload_id: &str,
        part_number: u32,
        body: &[u8],
    ) -> GatewayHttpResponse {
        if !self.buckets.read().unwrap().contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            );
        }

        {
            let multiparts = self.multiparts.read().unwrap();
            if !multiparts.contains_key(upload_id) {
                return GatewayHttpResponse::error(
                    S3ErrorCode::NoSuchUpload,
                    "O upload multipart especificado não existe",
                    Some(upload_id.to_string()),
                );
            }
        }

        let encoded = match self.erasure.encode(body) {
            Ok(enc) => enc,
            Err(e) => {
                return GatewayHttpResponse::error(
                    S3ErrorCode::InternalError,
                    format!("Erro de codificação: {}", e),
                    None,
                );
            }
        };

        let total_shards = self.erasure.total_shards() as u32;
        let mut shard_pointers = Vec::new();

        for (i, shard_bytes) in encoded.all_shards().into_iter().enumerate() {
            let shard_id = Uuid::new_v4();
            let is_parity = i >= self.erasure.data_shards();

            let location = match self
                .storage
                .write_shard(shard_id, i as u32, total_shards, &shard_bytes)
            {
                Ok(loc) => loc,
                Err(e) => {
                    return GatewayHttpResponse::error(
                        S3ErrorCode::InternalError,
                        format!("Erro de I/O em disco: {}", e),
                        None,
                    );
                }
            };

            shard_pointers.push(ShardPointer {
                shard_id: ShardId(shard_id),
                shard_index: i as u32,
                is_parity,
                node_id: self.node_id,
                extent_id: location.extent_id,
                offset_in_extent: location.offset_in_extent,
                length: location.payload_length,
                blake3_checksum: location.checksum_blake3,
            });
        }

        let part_etag_hex = calculate_md5_etag(body);
        let part_etag = ETag::from_hex(&part_etag_hex);

        let part = PartManifest {
            part_number,
            size: body.len() as u64,
            etag: part_etag.clone(),
            shards: shard_pointers,
        };

        let mut multiparts = self.multiparts.write().unwrap();
        if let Some(active) = multiparts.get_mut(upload_id) {
            active.parts.insert(part_number, part);
        }

        let mut resp = GatewayHttpResponse::ok_xml(String::new());
        resp.headers.insert("ETag".to_string(), format!("\"{}\"", part_etag.as_str()));
        resp
    }

    fn handle_complete_multipart(
        &self,
        bucket: &str,
        key: &str,
        upload_id: &str,
        headers: &HashMap<String, String>,
    ) -> GatewayHttpResponse {
        let bucket_name = match BucketName::new(bucket) {
            Ok(b) => b,
            Err(_) => {
                return GatewayHttpResponse::error(
                    S3ErrorCode::InvalidBucketName,
                    "Nome de bucket inválido",
                    Some(bucket.to_string()),
                );
            }
        };

        let key_obj = match ObjectKey::new(key) {
            Ok(k) => k,
            Err(_) => {
                return GatewayHttpResponse::error(
                    S3ErrorCode::InvalidArgument,
                    "Chave de objeto inválida",
                    Some(key.to_string()),
                );
            }
        };

        let versioning_status = {
            let buckets = self.buckets.read().unwrap();
            match buckets.get(bucket) {
                Some(b) => b.versioning,
                None => {
                    return GatewayHttpResponse::error(
                        S3ErrorCode::NoSuchBucket,
                        "O bucket especificado não existe",
                        Some(bucket.to_string()),
                    );
                }
            }
        };

        let active_upload = {
            let mut multiparts = self.multiparts.write().unwrap();
            match multiparts.remove(upload_id) {
                Some(upload) => upload,
                None => {
                    return GatewayHttpResponse::error(
                        S3ErrorCode::NoSuchUpload,
                        "O upload multipart especificado não existe",
                        Some(upload_id.to_string()),
                    );
                }
            }
        };

        let mut sorted_parts: Vec<PartManifest> = active_upload.parts.into_values().collect();
        sorted_parts.sort_by_key(|p| p.part_number);

        if sorted_parts.is_empty() {
            return GatewayHttpResponse::error(
                S3ErrorCode::InvalidPart,
                "Nenhuma parte foi enviada para este upload multipart",
                Some(upload_id.to_string()),
            );
        }

        let total_size: u64 = sorted_parts.iter().map(|p| p.size).sum();

        let mut concatenated_hashes = Vec::new();
        for p in &sorted_parts {
            if let Ok(bytes) = hex::decode(p.etag.as_str()) {
                concatenated_hashes.extend_from_slice(&bytes);
            }
        }
        let final_etag_str = format!("{}-{}", calculate_md5_etag(&concatenated_hashes), sorted_parts.len());
        let final_etag = ETag::from_hex(&final_etag_str);

        let content_type = headers
            .get("content-type")
            .cloned()
            .unwrap_or_else(|| "application/octet-stream".to_string());

        let (version_id_str, version_id) = match versioning_status {
            BucketVersioningStatus::Enabled => {
                let vid = Uuid::now_v7().to_string();
                (vid.clone(), VersionId::new(vid))
            }
            _ => ("null".to_string(), VersionId::new("null")),
        };

        let metadata = ObjectMetadata {
            bucket: bucket_name,
            key: key_obj,
            version_id,
            size: total_size,
            etag: final_etag.clone(),
            content_type,
            storage_class: StorageClass::Standard,
            created_at: chrono::Utc::now(),
            user_metadata: HashMap::new(),
            merkle_root: [0u8; 32],
            is_delete_marker: false,
            is_latest: true,
        };

        let manifest = ObjectManifest::new_with_parts(
            metadata,
            self.erasure.data_shards() as u32,
            self.erasure.parity_shards() as u32,
            sorted_parts,
        );

        let manifest_key = format!("{}/{}", bucket, key);
        let mut map = self.objects.write().unwrap();
        let version_list = map.entry(manifest_key).or_insert_with(Vec::new);

        for v in version_list.iter_mut() {
            v.metadata.is_latest = false;
        }

        if versioning_status == BucketVersioningStatus::Off {
            version_list.clear();
        }

        version_list.insert(0, manifest);
        drop(map);
        self.persist_objects();

        let mut resp = GatewayHttpResponse::ok_xml(
            CompleteMultipartUploadResult::new(bucket, key, final_etag.as_str()).to_xml(),
        );
        if versioning_status == BucketVersioningStatus::Enabled {
            resp.headers.insert("x-amz-version-id".to_string(), version_id_str);
        }
        resp.headers.insert("ETag".to_string(), format!("\"{}\"", final_etag.as_str()));
        resp
    }

    fn handle_abort_multipart(
        &self,
        bucket: &str,
        _key: &str,
        upload_id: &str,
    ) -> GatewayHttpResponse {
        if !self.buckets.read().unwrap().contains_key(bucket) {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchBucket,
                "O bucket especificado não existe",
                Some(bucket.to_string()),
            );
        }

        let mut multiparts = self.multiparts.write().unwrap();
        if multiparts.remove(upload_id).is_none() {
            return GatewayHttpResponse::error(
                S3ErrorCode::NoSuchUpload,
                "O upload multipart especificado não existe",
                Some(upload_id.to_string()),
            );
        }

        let mut headers = HashMap::new();
        headers.insert("Content-Length".to_string(), "0".to_string());
        GatewayHttpResponse {
            status: 204,
            headers,
            body: Bytes::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use z3s_auth::credentials::InMemoryCredentialsStore;

    #[test]
    fn test_s3_gateway_full_lifecycle_with_copy_and_delete_objects() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(StorageEngine::open(temp_dir.path(), 10 * 1024 * 1024).unwrap());
        let erasure = Arc::new(ErasureEngine::new(4, 2).unwrap());
        let credentials = Arc::new(InMemoryCredentialsStore::new());

        let service = S3GatewayService::new(Uuid::new_v4(), storage, erasure, credentials);
        let headers = HashMap::new();

        // 1. Create Bucket
        let create_resp = service.handle_request("PUT", "/empresa-data", None, &headers, &[]);
        assert_eq!(create_resp.status, 200);

        // 2. Put Object inicial
        let payload = b"Arquivo original antes do rename";
        let put_resp = service.handle_request("PUT", "/empresa-data/antigo.txt", None, &headers, payload);
        assert_eq!(put_resp.status, 200);

        // 3. CopyObject (Move/Rename simulado: PUT novo com x-amz-copy-source)
        let mut copy_headers = HashMap::new();
        copy_headers.insert("x-amz-copy-source".to_string(), "/empresa-data/antigo.txt".to_string());
        let copy_resp = service.handle_request("PUT", "/empresa-data/novo.txt", None, &copy_headers, &[]);
        assert_eq!(copy_resp.status, 200);
        let copy_xml = std::str::from_utf8(&copy_resp.body).unwrap();
        assert!(copy_xml.contains("<CopyObjectResult"));

        // 4. Valida que o novo arquivo contem os dados copiados
        let get_novo = service.handle_request("GET", "/empresa-data/novo.txt", None, &headers, &[]);
        assert_eq!(get_novo.status, 200);
        assert_eq!(get_novo.body.as_ref(), payload);

        // 5. DeleteObjects (Multi-delete)
        let delete_xml = b"<Delete><Object><Key>antigo.txt</Key></Object><Object><Key>novo.txt</Key></Object></Delete>";
        let del_multi = service.handle_request("POST", "/empresa-data", Some("delete"), &headers, delete_xml);
        assert_eq!(del_multi.status, 200);
        let del_res_xml = std::str::from_utf8(&del_multi.body).unwrap();
        assert!(del_res_xml.contains("<Deleted><Key>antigo.txt</Key></Deleted>"));
        assert!(del_res_xml.contains("<Deleted><Key>novo.txt</Key></Deleted>"));
    }

    #[test]
    fn test_s3_virtual_folders_and_subresources() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(StorageEngine::open(temp_dir.path(), 10 * 1024 * 1024).unwrap());
        let erasure = Arc::new(ErasureEngine::new(4, 2).unwrap());
        let credentials = Arc::new(InMemoryCredentialsStore::new());

        let service = S3GatewayService::new(Uuid::new_v4(), storage, erasure, credentials);
        let headers = HashMap::new();

        // 1. Cria bucket
        let create_resp = service.handle_request("PUT", "/meubucket", None, &headers, &[]);
        assert_eq!(create_resp.status, 200);

        // 2. Valida subresources de bucket (versioning, acl, location, uploads)
        let loc_resp = service.handle_request("GET", "/meubucket", Some("location"), &headers, &[]);
        assert_eq!(loc_resp.status, 200);
        assert!(std::str::from_utf8(&loc_resp.body).unwrap().contains("LocationConstraint"));

        let ver_resp = service.handle_request("GET", "/meubucket/", Some("versioning"), &headers, &[]);
        assert_eq!(ver_resp.status, 200);
        assert!(std::str::from_utf8(&ver_resp.body).unwrap().contains("VersioningConfiguration"));

        let acl_resp = service.handle_request("GET", "/meubucket", Some("acl"), &headers, &[]);
        assert_eq!(acl_resp.status, 200);
        assert!(std::str::from_utf8(&acl_resp.body).unwrap().contains("AccessControlPolicy"));

        let uploads_resp = service.handle_request("GET", "/meubucket", Some("uploads"), &headers, &[]);
        assert_eq!(uploads_resp.status, 200);
        assert!(std::str::from_utf8(&uploads_resp.body).unwrap().contains("ListMultipartUploadsResult"));

        // 3. Cria uma pasta virtual 'olde/' (PUT 0 bytes terminando com /)
        let folder_resp = service.handle_request("PUT", "/meubucket/olde/", None, &headers, &[]);
        assert_eq!(folder_resp.status, 200);

        // 4. Cria um arquivo dentro da pasta 'olde/Addresses.CDB'
        let file_payload = b"Sample CDB Content";
        let file_resp = service.handle_request("PUT", "/meubucket/olde/Addresses.CDB", None, &headers, file_payload);
        assert_eq!(file_resp.status, 200);

        // 5. Listagem na raiz com delimiter=/ e trailing slash na URI da requisição (/meubucket/)
        let list_root = service.handle_request("GET", "/meubucket/", Some("delimiter=/&prefix="), &headers, &[]);
        assert_eq!(list_root.status, 200);
        let list_root_xml = std::str::from_utf8(&list_root.body).unwrap();
        assert!(list_root_xml.contains("<CommonPrefixes><Prefix>olde/</Prefix></CommonPrefixes>"));

        // 6. Listagem dentro da pasta 'olde/' com delimiter=/ e prefix=olde/
        let list_folder = service.handle_request("GET", "/meubucket", Some("delimiter=/&prefix=olde/"), &headers, &[]);
        assert_eq!(list_folder.status, 200);
        let list_folder_xml = std::str::from_utf8(&list_folder.body).unwrap();
        assert!(list_folder_xml.contains("<Key>olde/Addresses.CDB</Key>"));
        // Não deve conter a própria pasta como um arquivo dentro de si mesma
        assert!(!list_folder_xml.contains("<Contents><Key>olde/</Key>"));

        // 7. Head e Get no arquivo dentro da pasta
        let head_resp = service.handle_request("HEAD", "/meubucket/olde/Addresses.CDB", None, &headers, &[]);
        assert_eq!(head_resp.status, 200);
        assert_eq!(head_resp.headers.get("Content-Length").unwrap(), &file_payload.len().to_string());

        let get_resp = service.handle_request("GET", "/meubucket/olde/Addresses.CDB", None, &headers, &[]);
        assert_eq!(get_resp.status, 200);
        assert_eq!(get_resp.body.as_ref(), file_payload);
    }

    #[test]
    fn test_s3_user_sequence_move_to_root_and_delete_folder() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(StorageEngine::open(temp_dir.path(), 10 * 1024 * 1024).unwrap());
        let erasure = Arc::new(ErasureEngine::new(4, 2).unwrap());
        let credentials = Arc::new(InMemoryCredentialsStore::new());

        let service = S3GatewayService::new_with_metadata(
            Uuid::new_v4(),
            storage,
            erasure,
            credentials,
            Some(temp_dir.path().join("metadata")),
        );
        let headers = HashMap::new();

        // 1. Novo bucket criado com sucesso
        let create_bucket = service.handle_request("PUT", "/novobucket", None, &headers, &[]);
        assert_eq!(create_bucket.status, 200);

        // 2. Pasta1 criada dentro do novo bucket
        let create_pasta1 = service.handle_request("PUT", "/novobucket/pasta1/", None, &headers, &[]);
        assert_eq!(create_pasta1.status, 200);

        // 3. Envio de arquivo para a pasta1
        let payload = b"Conteudo do arquivo de teste na pasta1";
        let upload_file = service.handle_request("PUT", "/novobucket/pasta1/meuarquivo.dat", None, &headers, payload);
        assert_eq!(upload_file.status, 200);

        // 4. Mover arquivo da pasta1 para raiz do bucket (PUT com x-amz-copy-source + DELETE)
        let mut copy_headers = HashMap::new();
        copy_headers.insert("x-amz-copy-source".to_string(), "/novobucket/pasta1%2Fmeuarquivo.dat".to_string());
        let copy_to_root = service.handle_request("PUT", "/novobucket/meuarquivo.dat", None, &copy_headers, &[]);
        assert_eq!(copy_to_root.status, 200);

        // Delete origem
        let del_origem = service.handle_request("DELETE", "/novobucket/pasta1/meuarquivo.dat", None, &headers, &[]);
        assert_eq!(del_origem.status, 204);

        // Valida que o arquivo está na raiz do bucket
        let get_root_file = service.handle_request("GET", "/novobucket/meuarquivo.dat", None, &headers, &[]);
        assert_eq!(get_root_file.status, 200);
        assert_eq!(get_root_file.body.as_ref(), payload);

        // 5. Criar uma nova pasta chamada delete e apagar ela
        let create_delete_folder = service.handle_request("PUT", "/novobucket/delete/", None, &headers, &[]);
        assert_eq!(create_delete_folder.status, 200);

        // Listagem deve conter a pasta delete
        let list_before = service.handle_request("GET", "/novobucket", Some("delimiter=/&prefix="), &headers, &[]);
        assert_eq!(list_before.status, 200);
        assert!(std::str::from_utf8(&list_before.body).unwrap().contains("<Prefix>delete/</Prefix>"));

        // Apaga a pasta delete via POST ?delete (DeleteObjects)
        let delete_xml = b"<Delete><Object><Key>delete/</Key></Object></Delete>";
        let del_folder = service.handle_request("POST", "/novobucket", Some("delete"), &headers, delete_xml);
        assert_eq!(del_folder.status, 200);

        // Listagem não deve mais conter a pasta delete
        let list_after = service.handle_request("GET", "/novobucket", Some("delimiter=/&prefix="), &headers, &[]);
        assert_eq!(list_after.status, 200);
        assert!(!std::str::from_utf8(&list_after.body).unwrap().contains("<Prefix>delete/</Prefix>"));

        // 6. Deletar os objetos restantes e o bucket inteiro
        let del_root_file = service.handle_request("DELETE", "/novobucket/meuarquivo.dat", None, &headers, &[]);
        assert_eq!(del_root_file.status, 204);
        let del_pasta1 = service.handle_request("DELETE", "/novobucket/pasta1/", None, &headers, &[]);
        assert_eq!(del_pasta1.status, 204);

        let del_bucket = service.handle_request("DELETE", "/novobucket", None, &headers, &[]);
        assert_eq!(del_bucket.status, 204);
    }

    #[test]
    fn test_s3_object_and_bucket_versioning_lifecycle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let storage = Arc::new(StorageEngine::open(temp_dir.path(), 10 * 1024 * 1024).unwrap());
        let erasure = Arc::new(ErasureEngine::new(4, 2).unwrap());
        let credentials = Arc::new(InMemoryCredentialsStore::new());

        let service = S3GatewayService::new_with_metadata(
            Uuid::new_v4(),
            storage,
            erasure,
            credentials,
            Some(temp_dir.path().join("metadata")),
        );
        let headers = HashMap::new();

        // 1. Cria bucket
        let create_bucket = service.handle_request("PUT", "/versioned-bucket", None, &headers, &[]);
        assert_eq!(create_bucket.status, 200);

        // 2. Consulta versionamento inicial (Off)
        let ver_initial = service.handle_request("GET", "/versioned-bucket", Some("versioning"), &headers, &[]);
        assert_eq!(ver_initial.status, 200);
        assert!(!std::str::from_utf8(&ver_initial.body).unwrap().contains("<Status>"));

        // 3. Ativa versionamento via PUT ?versioning
        let enable_xml = b"<VersioningConfiguration><Status>Enabled</Status></VersioningConfiguration>";
        let put_ver = service.handle_request("PUT", "/versioned-bucket", Some("versioning"), &headers, enable_xml);
        assert_eq!(put_ver.status, 200);

        // 4. Valida status Enabled
        let ver_enabled = service.handle_request("GET", "/versioned-bucket", Some("versioning"), &headers, &[]);
        assert_eq!(ver_enabled.status, 200);
        assert!(std::str::from_utf8(&ver_enabled.body).unwrap().contains("<Status>Enabled</Status>"));

        // 5. Upload Versão 1
        let payload_v1 = b"Versao 1 do documento importante";
        let put_v1 = service.handle_request("PUT", "/versioned-bucket/doc.txt", None, &headers, payload_v1);
        assert_eq!(put_v1.status, 200);
        let v1_id = put_v1.headers.get("x-amz-version-id").cloned().expect("Deve retornar version id");
        assert_ne!(v1_id, "null");

        // 6. Upload Versão 2
        let payload_v2 = b"Versao 2 com alteracoes adicionais";
        let put_v2 = service.handle_request("PUT", "/versioned-bucket/doc.txt", None, &headers, payload_v2);
        assert_eq!(put_v2.status, 200);
        let v2_id = put_v2.headers.get("x-amz-version-id").cloned().expect("Deve retornar version id");
        assert_ne!(v2_id, "null");
        assert_ne!(v1_id, v2_id);

        // 7. GET sem versionId retorna a mais recente (v2)
        let get_latest = service.handle_request("GET", "/versioned-bucket/doc.txt", None, &headers, &[]);
        assert_eq!(get_latest.status, 200);
        assert_eq!(get_latest.body.as_ref(), payload_v2);
        assert_eq!(get_latest.headers.get("x-amz-version-id").unwrap(), &v2_id);

        // 8. GET com versionId=v1 retorna a versão histórica
        let get_v1 = service.handle_request("GET", "/versioned-bucket/doc.txt", Some(&format!("versionId={}", v1_id)), &headers, &[]);
        assert_eq!(get_v1.status, 200);
        assert_eq!(get_v1.body.as_ref(), payload_v1);
        assert_eq!(get_v1.headers.get("x-amz-version-id").unwrap(), &v1_id);

        // 9. ListObjectVersions (GET ?versions)
        let list_ver = service.handle_request("GET", "/versioned-bucket", Some("versions"), &headers, &[]);
        assert_eq!(list_ver.status, 200);
        let list_xml = std::str::from_utf8(&list_ver.body).unwrap();
        assert!(list_xml.contains(&format!("<VersionId>{}</VersionId>", v1_id)));
        assert!(list_xml.contains(&format!("<VersionId>{}</VersionId>", v2_id)));
        assert!(list_xml.contains("<ListVersionsResult"));

        // 10. DELETE sem versionId cria Delete Marker
        let del_obj = service.handle_request("DELETE", "/versioned-bucket/doc.txt", None, &headers, &[]);
        assert_eq!(del_obj.status, 204);
        assert_eq!(del_obj.headers.get("x-amz-delete-marker").map(|s| s.as_str()), Some("true"));
        let marker_id = del_obj.headers.get("x-amz-version-id").cloned().expect("Deve retornar marker version id");

        // 11. GET normal agora retorna 404 NoSuchKey
        let get_after_del = service.handle_request("GET", "/versioned-bucket/doc.txt", None, &headers, &[]);
        assert_eq!(get_after_del.status, 404);

        // 12. ListObjectVersions mostra o DeleteMarker e as duas versões
        let list_ver2 = service.handle_request("GET", "/versioned-bucket", Some("versions"), &headers, &[]);
        assert_eq!(list_ver2.status, 200);
        let list2_xml = std::str::from_utf8(&list_ver2.body).unwrap();
        assert!(list2_xml.contains("<DeleteMarker>"));
        assert!(list2_xml.contains(&format!("<VersionId>{}</VersionId>", marker_id)));

        // 13. GET da versão 2 ainda funciona mesmo com o Delete Marker como topo
        let get_v2_explicit = service.handle_request("GET", "/versioned-bucket/doc.txt", Some(&format!("versionId={}", v2_id)), &headers, &[]);
        assert_eq!(get_v2_explicit.status, 200);
        assert_eq!(get_v2_explicit.body.as_ref(), payload_v2);

        // 14. Excluir o Delete Marker
        let del_marker = service.handle_request("DELETE", "/versioned-bucket/doc.txt", Some(&format!("versionId={}", marker_id)), &headers, &[]);
        assert_eq!(del_marker.status, 204);
        assert_eq!(del_marker.headers.get("x-amz-delete-marker").map(|s| s.as_str()), Some("true"));

        // 15. Objeto reaparece no GET normal (apontando para v2)
        let get_restored = service.handle_request("GET", "/versioned-bucket/doc.txt", None, &headers, &[]);
        assert_eq!(get_restored.status, 200);
        assert_eq!(get_restored.body.as_ref(), payload_v2);

        // 16. Excluir permanentemente v1 e v2
        let del_v1 = service.handle_request("DELETE", "/versioned-bucket/doc.txt", Some(&format!("versionId={}", v1_id)), &headers, &[]);
        assert_eq!(del_v1.status, 204);
        let del_v2 = service.handle_request("DELETE", "/versioned-bucket/doc.txt", Some(&format!("versionId={}", v2_id)), &headers, &[]);
        assert_eq!(del_v2.status, 204);

        // 17. Bucket vazio pode ser deletado
        let del_bucket = service.handle_request("DELETE", "/versioned-bucket", None, &headers, &[]);
        assert_eq!(del_bucket.status, 204);
    }
}
