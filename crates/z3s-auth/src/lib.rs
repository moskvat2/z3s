//! # z3s-auth
//!
//! Autenticação AWS SigV4, URLs pré-assinadas e motor de avaliação de políticas IAM.

pub mod credentials;
pub mod sigv4;

pub use credentials::{Credentials, CredentialsProvider, InMemoryCredentialsStore};
pub use sigv4::{SigV4AuthContext, SigV4Engine, SigV4Error, SIGV4_ALGORITHM, UNSIGNED_PAYLOAD};
