use quick_xml::se::to_string;
use serde::{Deserialize, Serialize};
use z3s_common::S3ErrorCode;

/// Resposta de erro oficial do AWS S3 em formato XML
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename = "Error")]
pub struct S3XmlError {
    #[serde(rename = "Code")]
    pub code: String,
    #[serde(rename = "Message")]
    pub message: String,
    #[serde(rename = "Resource", skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
    #[serde(rename = "RequestId")]
    pub request_id: String,
}

impl S3XmlError {
    pub fn new(code: S3ErrorCode, message: impl Into<String>, resource: Option<String>) -> Self {
        Self {
            code: code.as_str().to_string(),
            message: message.into(),
            resource,
            request_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn to_xml(&self) -> String {
        let body = to_string(self).unwrap_or_else(|_| {
            format!(
                "<Error><Code>{}</Code><Message>{}</Message></Error>",
                self.code, self.message
            )
        });
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", body)
    }
}

/// Resposta de localização de bucket (GetBucketLocation)
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename = "LocationConstraint")]
pub struct LocationConstraint {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "$value")]
    pub location: &'static str,
}

impl LocationConstraint {
    pub fn new(region: &'static str) -> Self {
        Self {
            xmlns: "http://s3.amazonaws.com/doc/2006-03-01/",
            location: region,
        }
    }

    pub fn to_xml(&self) -> String {
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<LocationConstraint xmlns=\"{}\">{}</LocationConstraint>",
            self.xmlns, self.location
        )
    }
}

/// Item de Bucket na listagem de buckets do proprietário
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BucketItem {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "CreationDate")]
    pub creation_date: String,
}

/// Resposta de ListBuckets (GET /)
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename = "ListAllMyBucketsResult")]
pub struct ListAllMyBucketsResult {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "Buckets")]
    pub buckets: BucketsWrapper,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BucketsWrapper {
    #[serde(rename = "Bucket")]
    pub bucket: Vec<BucketItem>,
}

impl ListAllMyBucketsResult {
    pub fn new(buckets: Vec<BucketItem>) -> Self {
        Self {
            xmlns: "http://s3.amazonaws.com/doc/2006-03-01/",
            buckets: BucketsWrapper { bucket: buckets },
        }
    }

    pub fn to_xml(&self) -> String {
        let body = to_string(self).unwrap();
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", body)
    }
}

/// Item de Objeto na listagem de objetos (ListObjectsV2)
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ObjectItem {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "LastModified")]
    pub last_modified: String,
    #[serde(rename = "ETag")]
    pub etag: String,
    #[serde(rename = "Size")]
    pub size: u64,
    #[serde(rename = "StorageClass")]
    pub storage_class: String,
}

/// Prefixo Comum (Emulação de pastas no S3)
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommonPrefixItem {
    #[serde(rename = "Prefix")]
    pub prefix: String,
}

/// Resposta de ListObjectsV2 (GET /bucket?list-type=2)
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename = "ListBucketResult")]
pub struct ListBucketResult {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Prefix")]
    pub prefix: String,
    #[serde(rename = "Delimiter", skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    #[serde(rename = "KeyCount")]
    pub key_count: usize,
    #[serde(rename = "MaxKeys")]
    pub max_keys: usize,
    #[serde(rename = "IsTruncated")]
    pub is_truncated: bool,
    #[serde(rename = "Contents", default, skip_serializing_if = "Vec::is_empty")]
    pub contents: Vec<ObjectItem>,
    #[serde(rename = "CommonPrefixes", default, skip_serializing_if = "Vec::is_empty")]
    pub common_prefixes: Vec<CommonPrefixItem>,
}

impl ListBucketResult {
    pub fn new(
        name: String,
        prefix: String,
        delimiter: Option<String>,
        max_keys: usize,
        contents: Vec<ObjectItem>,
        common_prefixes: Vec<CommonPrefixItem>,
    ) -> Self {
        let key_count = contents.len() + common_prefixes.len();
        Self {
            xmlns: "http://s3.amazonaws.com/doc/2006-03-01/",
            name,
            prefix,
            delimiter,
            key_count,
            max_keys,
            is_truncated: false,
            contents,
            common_prefixes,
        }
    }

    pub fn to_xml(&self) -> String {
        let body = to_string(self).unwrap();
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", body)
    }
}

/// Resposta de CopyObject (PUT /bucket/new-key com x-amz-copy-source)
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename = "CopyObjectResult")]
pub struct CopyObjectResult {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "LastModified")]
    pub last_modified: String,
    #[serde(rename = "ETag")]
    pub etag: String,
}

impl CopyObjectResult {
    pub fn new(last_modified: String, etag: String) -> Self {
        Self {
            xmlns: "http://s3.amazonaws.com/doc/2006-03-01/",
            last_modified,
            etag,
        }
    }

    pub fn to_xml(&self) -> String {
        let body = to_string(self).unwrap();
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", body)
    }
}

/// Item excluído em DeleteObjects (Multi-Object Delete)
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeletedItem {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "VersionId", skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    #[serde(rename = "DeleteMarker", skip_serializing_if = "Option::is_none")]
    pub delete_marker: Option<bool>,
    #[serde(rename = "DeleteMarkerVersionId", skip_serializing_if = "Option::is_none")]
    pub delete_marker_version_id: Option<String>,
}

/// Resposta de DeleteObjects (POST /bucket?delete)
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename = "DeleteResult")]
pub struct DeleteResult {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "Deleted", default, skip_serializing_if = "Vec::is_empty")]
    pub deleted: Vec<DeletedItem>,
}

impl DeleteResult {
    pub fn new(deleted: Vec<DeletedItem>) -> Self {
        Self {
            xmlns: "http://s3.amazonaws.com/doc/2006-03-01/",
            deleted,
        }
    }

    pub fn to_xml(&self) -> String {
        let body = to_string(self).unwrap();
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", body)
    }
}

/// Resposta de InitiateMultipartUpload (POST /bucket/object?uploads)
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename = "InitiateMultipartUploadResult")]
pub struct InitiateMultipartUploadResult {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "Bucket")]
    pub bucket: String,
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "UploadId")]
    pub upload_id: String,
}

impl InitiateMultipartUploadResult {
    pub fn new(bucket: impl Into<String>, key: impl Into<String>, upload_id: impl Into<String>) -> Self {
        Self {
            xmlns: "http://s3.amazonaws.com/doc/2006-03-01/",
            bucket: bucket.into(),
            key: key.into(),
            upload_id: upload_id.into(),
        }
    }

    pub fn to_xml(&self) -> String {
        let body = to_string(self).unwrap();
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", body)
    }
}

/// Resposta de CompleteMultipartUpload (POST /bucket/object?uploadId=X)
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename = "CompleteMultipartUploadResult")]
pub struct CompleteMultipartUploadResult {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "Location")]
    pub location: String,
    #[serde(rename = "Bucket")]
    pub bucket: String,
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "ETag")]
    pub etag: String,
}

impl CompleteMultipartUploadResult {
    pub fn new(bucket: impl Into<String>, key: impl Into<String>, etag: impl Into<String>) -> Self {
        let b = bucket.into();
        let k = key.into();
        Self {
            xmlns: "http://s3.amazonaws.com/doc/2006-03-01/",
            location: format!("/{}/{}", b, k),
            bucket: b,
            key: k,
            etag: etag.into(),
        }
    }

    pub fn to_xml(&self) -> String {
        let body = to_string(self).unwrap();
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", body)
    }
}

/// Configuração de Versionamento de Bucket (GetBucketVersioning)
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename = "VersioningConfiguration")]
pub struct VersioningConfiguration {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "Status", skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

impl VersioningConfiguration {
    pub fn new(status: Option<String>) -> Self {
        Self {
            xmlns: "http://s3.amazonaws.com/doc/2006-03-01/",
            status,
        }
    }

    pub fn to_xml(&self) -> String {
        let body = to_string(self).unwrap();
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", body)
    }
}

/// Política de Controle de Acesso (GetBucketAcl / GetObjectAcl)
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename = "AccessControlPolicy")]
pub struct AccessControlPolicy {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "Owner")]
    pub owner: AclOwner,
    #[serde(rename = "AccessControlList")]
    pub access_control_list: AccessControlList,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AclOwner {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "DisplayName")]
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AccessControlList {
    #[serde(rename = "Grant")]
    pub grants: Vec<AclGrant>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AclGrant {
    #[serde(rename = "Grantee")]
    pub grantee: AclGrantee,
    #[serde(rename = "Permission")]
    pub permission: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AclGrantee {
    #[serde(rename = "@xmlns:xsi")]
    pub xmlns_xsi: &'static str,
    #[serde(rename = "@xsi:type")]
    pub xsi_type: &'static str,
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "DisplayName")]
    pub display_name: String,
}

impl AccessControlPolicy {
    pub fn default_owner(owner_id: impl Into<String>) -> Self {
        let id = owner_id.into();
        Self {
            xmlns: "http://s3.amazonaws.com/doc/2006-03-01/",
            owner: AclOwner {
                id: id.clone(),
                display_name: id.clone(),
            },
            access_control_list: AccessControlList {
                grants: vec![AclGrant {
                    grantee: AclGrantee {
                        xmlns_xsi: "http://www.w3.org/2001/XMLSchema-instance",
                        xsi_type: "CanonicalUser",
                        id: id.clone(),
                        display_name: id,
                    },
                    permission: "FULL_CONTROL".to_string(),
                }],
            },
        }
    }

    pub fn to_xml(&self) -> String {
        let body = to_string(self).unwrap();
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", body)
    }
}

/// Resposta de ListMultipartUploads (GET /bucket?uploads)
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename = "ListMultipartUploadsResult")]
pub struct ListMultipartUploadsResult {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "Bucket")]
    pub bucket: String,
    #[serde(rename = "KeyMarker")]
    pub key_marker: String,
    #[serde(rename = "UploadIdMarker")]
    pub upload_id_marker: String,
    #[serde(rename = "NextKeyMarker")]
    pub next_key_marker: String,
    #[serde(rename = "NextUploadIdMarker")]
    pub next_upload_id_marker: String,
    #[serde(rename = "MaxUploads")]
    pub max_uploads: usize,
    #[serde(rename = "IsTruncated")]
    pub is_truncated: bool,
}

impl ListMultipartUploadsResult {
    pub fn new(bucket: impl Into<String>) -> Self {
        Self {
            xmlns: "http://s3.amazonaws.com/doc/2006-03-01/",
            bucket: bucket.into(),
            key_marker: String::new(),
            upload_id_marker: String::new(),
            next_key_marker: String::new(),
            next_upload_id_marker: String::new(),
            max_uploads: 1000,
            is_truncated: false,
        }
    }

    pub fn to_xml(&self) -> String {
        let body = to_string(self).unwrap();
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", body)
    }
}

/// Item de versão em ListVersionsResult
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VersionItem {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "VersionId")]
    pub version_id: String,
    #[serde(rename = "IsLatest")]
    pub is_latest: bool,
    #[serde(rename = "LastModified")]
    pub last_modified: String,
    #[serde(rename = "ETag")]
    pub etag: String,
    #[serde(rename = "Size")]
    pub size: u64,
    #[serde(rename = "Owner")]
    pub owner: AclOwner,
    #[serde(rename = "StorageClass")]
    pub storage_class: String,
}

/// Item de marcador de deleção em ListVersionsResult
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeleteMarkerItem {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "VersionId")]
    pub version_id: String,
    #[serde(rename = "IsLatest")]
    pub is_latest: bool,
    #[serde(rename = "LastModified")]
    pub last_modified: String,
    #[serde(rename = "Owner")]
    pub owner: AclOwner,
}

/// Resposta de ListObjectVersions (GET /bucket?versions)
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename = "ListVersionsResult")]
pub struct ListVersionsResult {
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Prefix")]
    pub prefix: String,
    #[serde(rename = "Delimiter", skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    #[serde(rename = "KeyMarker")]
    pub key_marker: String,
    #[serde(rename = "VersionIdMarker")]
    pub version_id_marker: String,
    #[serde(rename = "MaxKeys")]
    pub max_keys: usize,
    #[serde(rename = "IsTruncated")]
    pub is_truncated: bool,
    #[serde(rename = "Version", default, skip_serializing_if = "Vec::is_empty")]
    pub versions: Vec<VersionItem>,
    #[serde(rename = "DeleteMarker", default, skip_serializing_if = "Vec::is_empty")]
    pub delete_markers: Vec<DeleteMarkerItem>,
    #[serde(rename = "CommonPrefixes", default, skip_serializing_if = "Vec::is_empty")]
    pub common_prefixes: Vec<CommonPrefixItem>,
}

impl ListVersionsResult {
    pub fn new(
        name: impl Into<String>,
        prefix: impl Into<String>,
        delimiter: Option<String>,
        versions: Vec<VersionItem>,
        delete_markers: Vec<DeleteMarkerItem>,
        common_prefixes: Vec<CommonPrefixItem>,
    ) -> Self {
        Self {
            xmlns: "http://s3.amazonaws.com/doc/2006-03-01/",
            name: name.into(),
            prefix: prefix.into(),
            delimiter,
            key_marker: String::new(),
            version_id_marker: String::new(),
            max_keys: 1000,
            is_truncated: false,
            versions,
            delete_markers,
            common_prefixes,
        }
    }

    pub fn to_xml(&self) -> String {
        let body = to_string(self).unwrap();
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", body)
    }
}

/// Configuração de Criptografia em Repouso do Bucket (GetBucketEncryption / PutBucketEncryption)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename = "ServerSideEncryptionConfiguration")]
pub struct ServerSideEncryptionConfiguration {
    #[serde(rename = "@xmlns", default = "default_s3_xmlns")]
    pub xmlns: String,
    #[serde(rename = "Rule")]
    pub rule: ServerSideEncryptionRule,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerSideEncryptionRule {
    #[serde(rename = "ApplyServerSideEncryptionByDefault")]
    pub apply_server_side_encryption_by_default: ApplyServerSideEncryptionByDefault,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApplyServerSideEncryptionByDefault {
    #[serde(rename = "SSEAlgorithm")]
    pub sse_algorithm: String, // "AES256" ou "aws:kms"
    #[serde(rename = "KMSMasterKeyID", skip_serializing_if = "Option::is_none")]
    pub kms_master_key_id: Option<String>,
}

fn default_s3_xmlns() -> String {
    "http://s3.amazonaws.com/doc/2006-03-01/".to_string()
}

impl ServerSideEncryptionConfiguration {
    pub fn new_aes256() -> Self {
        Self {
            xmlns: default_s3_xmlns(),
            rule: ServerSideEncryptionRule {
                apply_server_side_encryption_by_default: ApplyServerSideEncryptionByDefault {
                    sse_algorithm: "AES256".to_string(),
                    kms_master_key_id: None,
                },
            },
        }
    }

    pub fn new_kms(kms_key_id: Option<String>) -> Self {
        Self {
            xmlns: default_s3_xmlns(),
            rule: ServerSideEncryptionRule {
                apply_server_side_encryption_by_default: ApplyServerSideEncryptionByDefault {
                    sse_algorithm: "aws:kms".to_string(),
                    kms_master_key_id: kms_key_id,
                },
            },
        }
    }

    pub fn to_xml(&self) -> String {
        let body = to_string(self).unwrap();
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", body)
    }
}

/// Configuração de Ciclo de Vida do Bucket (GetBucketLifecycleConfiguration / PutBucketLifecycleConfiguration)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename = "LifecycleConfiguration")]
pub struct LifecycleConfiguration {
    #[serde(rename = "@xmlns", default = "default_s3_xmlns")]
    pub xmlns: String,
    #[serde(rename = "Rule")]
    pub rules: Vec<LifecycleRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleRule {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Status")]
    pub status: String, // "Enabled" | "Disabled"
    #[serde(rename = "Filter", default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<LifecycleFilter>,
    #[serde(rename = "Expiration", default, skip_serializing_if = "Option::is_none")]
    pub expiration: Option<LifecycleExpiration>,
    #[serde(rename = "Transition", default, skip_serializing_if = "Option::is_none")]
    pub transition: Option<LifecycleTransition>,
    #[serde(rename = "NoncurrentVersionExpiration", default, skip_serializing_if = "Option::is_none")]
    pub noncurrent_version_expiration: Option<NoncurrentVersionExpiration>,
    #[serde(rename = "AbortIncompleteMultipartUpload", default, skip_serializing_if = "Option::is_none")]
    pub abort_incomplete_multipart_upload: Option<AbortIncompleteMultipartUpload>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LifecycleFilter {
    #[serde(rename = "Prefix", default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleExpiration {
    #[serde(rename = "Days", default, skip_serializing_if = "Option::is_none")]
    pub days: Option<u32>,
    #[serde(rename = "Date", default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(rename = "ExpiredObjectDeleteMarker", default, skip_serializing_if = "Option::is_none")]
    pub expired_object_delete_marker: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LifecycleTransition {
    #[serde(rename = "Days", default, skip_serializing_if = "Option::is_none")]
    pub days: Option<u32>,
    #[serde(rename = "StorageClass")]
    pub storage_class: String, // "STANDARD_IA", "GLACIER", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoncurrentVersionExpiration {
    #[serde(rename = "NoncurrentDays")]
    pub noncurrent_days: u32,
    #[serde(rename = "NewerNoncurrentVersions", default, skip_serializing_if = "Option::is_none")]
    pub newer_noncurrent_versions: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AbortIncompleteMultipartUpload {
    #[serde(rename = "DaysAfterInitiation")]
    pub days_after_initiation: u32,
}

impl LifecycleConfiguration {
    pub fn new(rules: Vec<LifecycleRule>) -> Self {
        Self {
            xmlns: default_s3_xmlns(),
            rules,
        }
    }

    pub fn to_xml(&self) -> String {
        let body = to_string(self).unwrap();
        format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", body)
    }
}

