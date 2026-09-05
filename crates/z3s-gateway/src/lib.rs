//! # z3s-gateway
//!
//! S3 REST API Server, roteamento HTTP, pipeline de streaming e serialização XML.

pub mod gc;
pub mod lifecycle;
pub mod router;
pub mod server;
pub mod service;
pub mod xml;

pub use gc::{GarbageCollector, GcReport};
pub use lifecycle::{LifecycleEngine, LifecycleReport};
pub use router::{S3Action, S3Router};
pub use server::HttpServer;
pub use service::{GatewayHttpResponse, S3GatewayService};
pub use xml::{
    BucketItem, LifecycleConfiguration, LifecycleRule, ListAllMyBucketsResult, ListBucketResult,
    ObjectItem, S3XmlError, ServerSideEncryptionConfiguration,
};
