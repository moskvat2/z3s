use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Códigos de erro oficiais do Amazon S3 REST API
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum S3ErrorCode {
    AccessDenied,
    BucketAlreadyExists,
    BucketAlreadyOwnedByYou,
    BucketNotEmpty,
    EntityTooLarge,
    EntityTooSmall,
    InternalError,
    InvalidArgument,
    InvalidBucketName,
    InvalidDigest,
    InvalidPart,
    InvalidRange,
    MalformedXML,
    NoSuchBucket,
    NoSuchBucketPolicy,
    NoSuchCORSConfiguration,
    NoSuchKey,
    NoSuchLifecycleConfiguration,
    NoSuchPublicAccessBlockConfiguration,
    NoSuchTagSet,
    NoSuchUpload,
    NoSuchVersion,
    NotImplemented,
    ObjectLockConfigurationNotFoundError,
    PreconditionFailed,
    ServerSideEncryptionConfigurationNotFoundError,
    SignatureDoesNotMatch,
}

impl S3ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AccessDenied => "AccessDenied",
            Self::BucketAlreadyExists => "BucketAlreadyExists",
            Self::BucketAlreadyOwnedByYou => "BucketAlreadyOwnedByYou",
            Self::BucketNotEmpty => "BucketNotEmpty",
            Self::EntityTooLarge => "EntityTooLarge",
            Self::EntityTooSmall => "EntityTooSmall",
            Self::InternalError => "InternalError",
            Self::InvalidArgument => "InvalidArgument",
            Self::InvalidBucketName => "InvalidBucketName",
            Self::InvalidDigest => "InvalidDigest",
            Self::InvalidPart => "InvalidPart",
            Self::InvalidRange => "InvalidRange",
            Self::MalformedXML => "MalformedXML",
            Self::NoSuchBucket => "NoSuchBucket",
            Self::NoSuchBucketPolicy => "NoSuchBucketPolicy",
            Self::NoSuchCORSConfiguration => "NoSuchCORSConfiguration",
            Self::NoSuchKey => "NoSuchKey",
            Self::NoSuchLifecycleConfiguration => "NoSuchLifecycleConfiguration",
            Self::NoSuchPublicAccessBlockConfiguration => "NoSuchPublicAccessBlockConfiguration",
            Self::NoSuchTagSet => "NoSuchTagSet",
            Self::NoSuchUpload => "NoSuchUpload",
            Self::NoSuchVersion => "NoSuchVersion",
            Self::NotImplemented => "NotImplemented",
            Self::ObjectLockConfigurationNotFoundError => "ObjectLockConfigurationNotFoundError",
            Self::PreconditionFailed => "PreconditionFailed",
            Self::ServerSideEncryptionConfigurationNotFoundError => "ServerSideEncryptionConfigurationNotFoundError",
            Self::SignatureDoesNotMatch => "SignatureDoesNotMatch",
        }
    }

    pub fn http_status(&self) -> u16 {
        match self {
            Self::AccessDenied | Self::SignatureDoesNotMatch => 403,
            Self::NoSuchBucket
            | Self::NoSuchBucketPolicy
            | Self::NoSuchCORSConfiguration
            | Self::NoSuchKey
            | Self::NoSuchLifecycleConfiguration
            | Self::NoSuchPublicAccessBlockConfiguration
            | Self::NoSuchTagSet
            | Self::NoSuchUpload
            | Self::NoSuchVersion
            | Self::ObjectLockConfigurationNotFoundError
            | Self::ServerSideEncryptionConfigurationNotFoundError => 404,
            Self::BucketAlreadyExists | Self::BucketAlreadyOwnedByYou | Self::BucketNotEmpty => 409,
            Self::EntityTooLarge | Self::EntityTooSmall | Self::InvalidBucketName
            | Self::InvalidArgument | Self::InvalidDigest | Self::InvalidPart | Self::MalformedXML => 400,
            Self::InvalidRange => 416,
            Self::PreconditionFailed => 412,
            Self::NotImplemented => 501,
            Self::InternalError => 500,
        }
    }
}

/// Erro central do sistema S3
#[derive(Debug, Error)]
pub enum S3Error {
    #[error("S3 Error [{code:?}]: {message}")]
    S3 {
        code: S3ErrorCode,
        message: String,
        resource: Option<String>,
        request_id: Option<String>,
    },

    #[error("I/O Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type S3Result<T> = Result<T, S3Error>;
