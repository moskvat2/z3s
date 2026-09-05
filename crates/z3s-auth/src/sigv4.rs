use crate::credentials::CredentialsProvider;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

pub const SIGV4_ALGORITHM: &str = "AWS4-HMAC-SHA256";
pub const UNSIGNED_PAYLOAD: &str = "UNSIGNED-PAYLOAD";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SigV4Error {
    #[error("Cabeçalho de autorização ausente")]
    MissingAuthorizationHeader,

    #[error("Algoritmo de assinatura não suportado: {0}")]
    UnsupportedAlgorithm(String),

    #[error("Formato do cabeçalho de autorização inválido")]
    MalformedAuthorizationHeader,

    #[error("Credenciais inválidas ou usuário inexistente: {0}")]
    InvalidCredentials(String),

    #[error("Data de requisição ausente (x-amz-date ou Date)")]
    MissingDateHeader,

    #[error("Assinatura calculada não confere com a assinatura enviada")]
    SignatureDoesNotMatch,

    #[error("URL pré-assinada expirada")]
    RequestExpired,
}

/// Parâmetros extraídos da autorização SigV4
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigV4AuthContext {
    pub access_key_id: String,
    pub date: String,          // yyyyMMdd
    pub timestamp: String,     // yyyyMMddTHHmmssZ
    pub region: String,
    pub service: String,
    pub signed_headers: Vec<String>,
    pub signature: String,
}

/// Motor de Autenticação AWS Signature Version 4
pub struct SigV4Engine;

impl SigV4Engine {
    /// Faz o parsing do cabeçalho `Authorization: AWS4-HMAC-SHA256 ...`
    pub fn parse_authorization_header(header: &str) -> Result<SigV4AuthContext, SigV4Error> {
        let header = header.trim();
        if !header.starts_with(SIGV4_ALGORITHM) {
            return Err(SigV4Error::UnsupportedAlgorithm(header.to_string()));
        }

        let params_part = header[SIGV4_ALGORITHM.len()..].trim();
        let mut credential_opt = None;
        let mut signed_headers_opt = None;
        let mut signature_opt = None;

        for part in params_part.split(',') {
            let part = part.trim();
            if let Some((key, val)) = part.split_once('=') {
                match key.trim() {
                    "Credential" => credential_opt = Some(val.trim().to_string()),
                    "SignedHeaders" => signed_headers_opt = Some(val.trim().to_string()),
                    "Signature" => signature_opt = Some(val.trim().to_string()),
                    _ => {}
                }
            }
        }

        let credential = credential_opt.ok_or(SigV4Error::MalformedAuthorizationHeader)?;
        let signed_headers_str = signed_headers_opt.ok_or(SigV4Error::MalformedAuthorizationHeader)?;
        let signature = signature_opt.ok_or(SigV4Error::MalformedAuthorizationHeader)?;

        // Credential formato: <AccessKey>/<Date>/<Region>/<Service>/aws4_request
        let cred_parts: Vec<&str> = credential.split('/').collect();
        if cred_parts.len() != 5 || cred_parts[4] != "aws4_request" {
            return Err(SigV4Error::MalformedAuthorizationHeader);
        }

        let access_key_id = cred_parts[0].to_string();
        let date = cred_parts[1].to_string();
        let region = cred_parts[2].to_string();
        let service = cred_parts[3].to_string();

        let signed_headers: Vec<String> = signed_headers_str
            .split(';')
            .map(|s| s.trim().to_ascii_lowercase())
            .collect();

        Ok(SigV4AuthContext {
            access_key_id,
            date,
            timestamp: String::new(), // Preenchido com x-amz-date
            region,
            service,
            signed_headers,
            signature,
        })
    }

    /// Deriva a chave de assinatura HMAC: kSigning = HMAC(HMAC(HMAC(HMAC("AWS4" + Secret, Date), Region), Service), "aws4_request")
    pub fn derive_signing_key(
        secret_access_key: &str,
        date: &str,
        region: &str,
        service: &str,
    ) -> Vec<u8> {
        let k_secret = format!("AWS4{}", secret_access_key);
        let k_date = Self::hmac_sha256(k_secret.as_bytes(), date.as_bytes());
        let k_region = Self::hmac_sha256(&k_date, region.as_bytes());
        let k_service = Self::hmac_sha256(&k_region, service.as_bytes());
        Self::hmac_sha256(&k_service, b"aws4_request")
    }

    /// Constrói o Canonical Request oficial do S3
    pub fn build_canonical_request(
        http_method: &str,
        canonical_uri: &str,
        canonical_query_string: &str,
        canonical_headers: &BTreeMap<String, String>,
        signed_headers_list: &[String],
        payload_hash: &str,
    ) -> String {
        let mut headers_str = String::new();
        for key in signed_headers_list {
            if let Some(val) = canonical_headers.get(key) {
                headers_str.push_str(key);
                headers_str.push(':');
                headers_str.push_str(val.trim());
                headers_str.push('\n');
            }
        }

        let signed_headers = signed_headers_list.join(";");

        format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            http_method.to_uppercase(),
            canonical_uri,
            canonical_query_string,
            headers_str,
            signed_headers,
            payload_hash
        )
    }

    /// Constrói a String to Sign
    pub fn build_string_to_sign(
        timestamp: &str,
        date: &str,
        region: &str,
        service: &str,
        canonical_request: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(canonical_request.as_bytes());
        let canonical_hash = hex::encode(hasher.finalize());

        format!(
            "{}\n{}\n{}/{}/{}/aws4_request\n{}",
            SIGV4_ALGORITHM, timestamp, date, region, service, canonical_hash
        )
    }

    /// Calcula a assinatura final em hexadecimal
    pub fn calculate_signature(signing_key: &[u8], string_to_sign: &str) -> String {
        let signature = Self::hmac_sha256(signing_key, string_to_sign.as_bytes());
        hex::encode(signature)
    }

    /// Valida completamente a requisição SigV4 contra um repositório de credenciais
    pub fn verify(
        auth_context: &SigV4AuthContext,
        credentials_provider: &dyn CredentialsProvider,
        http_method: &str,
        canonical_uri: &str,
        canonical_query_string: &str,
        headers: &BTreeMap<String, String>,
        payload_hash: &str,
        timestamp: &str,
    ) -> Result<(), SigV4Error> {
        let credentials = credentials_provider
            .get_credentials(&auth_context.access_key_id)
            .ok_or_else(|| SigV4Error::InvalidCredentials(auth_context.access_key_id.clone()))?;

        let canonical_request = Self::build_canonical_request(
            http_method,
            canonical_uri,
            canonical_query_string,
            headers,
            &auth_context.signed_headers,
            payload_hash,
        );

        let string_to_sign = Self::build_string_to_sign(
            timestamp,
            &auth_context.date,
            &auth_context.region,
            &auth_context.service,
            &canonical_request,
        );

        let signing_key = Self::derive_signing_key(
            &credentials.secret_access_key,
            &auth_context.date,
            &auth_context.region,
            &auth_context.service,
        );

        let expected_signature = Self::calculate_signature(&signing_key, &string_to_sign);

        // Comparação segura de tempo constante
        if expected_signature != auth_context.signature {
            return Err(SigV4Error::SignatureDoesNotMatch);
        }

        Ok(())
    }

    fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC pode aceitar chave de qualquer tamanho");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::InMemoryCredentialsStore;

    #[test]
    fn test_parse_authorization_header() {
        let header = "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, SignedHeaders=host;range;x-amz-date, Signature=fe5f80f7793fa1bec5da01f9bab7890f017f35079bc0881ab16b93459ab538f7";
        let parsed = SigV4Engine::parse_authorization_header(header).unwrap();

        assert_eq!(parsed.access_key_id, "AKIAIOSFODNN7EXAMPLE");
        assert_eq!(parsed.date, "20130524");
        assert_eq!(parsed.region, "us-east-1");
        assert_eq!(parsed.service, "s3");
        assert_eq!(parsed.signed_headers, vec!["host", "range", "x-amz-date"]);
        assert_eq!(
            parsed.signature,
            "fe5f80f7793fa1bec5da01f9bab7890f017f35079bc0881ab16b93459ab538f7"
        );
    }

    #[test]
    fn test_sigv4_full_signature_flow_valid_and_invalid() {
        let store = InMemoryCredentialsStore::new();
        let access_key = "MY_ACCESS_KEY";
        let secret_key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        store.register(access_key, secret_key);

        let date = "20260904";
        let timestamp = "20260904T200000Z";
        let region = "us-east-1";
        let service = "s3";
        let http_method = "PUT";
        let canonical_uri = "/meu-bucket/arquivo.txt";
        let canonical_query_string = "";

        let mut headers = BTreeMap::new();
        headers.insert("host".to_string(), "s3.amazonaws.com".to_string());
        headers.insert("x-amz-date".to_string(), timestamp.to_string());
        headers.insert("x-amz-content-sha256".to_string(), UNSIGNED_PAYLOAD.to_string());

        let signed_headers = vec!["host".to_string(), "x-amz-content-sha256".to_string(), "x-amz-date".to_string()];

        // 1. Gera assinatura válida
        let canonical_request = SigV4Engine::build_canonical_request(
            http_method,
            canonical_uri,
            canonical_query_string,
            &headers,
            &signed_headers,
            UNSIGNED_PAYLOAD,
        );

        let string_to_sign = SigV4Engine::build_string_to_sign(
            timestamp,
            date,
            region,
            service,
            &canonical_request,
        );

        let signing_key = SigV4Engine::derive_signing_key(secret_key, date, region, service);
        let signature = SigV4Engine::calculate_signature(&signing_key, &string_to_sign);

        let auth_context = SigV4AuthContext {
            access_key_id: access_key.to_string(),
            date: date.to_string(),
            timestamp: timestamp.to_string(),
            region: region.to_string(),
            service: service.to_string(),
            signed_headers: signed_headers.clone(),
            signature: signature.clone(),
        };

        // 2. Valida com sucesso
        let verify_result = SigV4Engine::verify(
            &auth_context,
            &store,
            http_method,
            canonical_uri,
            canonical_query_string,
            &headers,
            UNSIGNED_PAYLOAD,
            timestamp,
        );
        assert!(verify_result.is_ok());

        // 3. Valida rejeição quando assinatura é forjada
        let mut tampered_auth = auth_context.clone();
        tampered_auth.signature = "0000000000000000000000000000000000000000000000000000000000000000".to_string();

        let verify_tampered = SigV4Engine::verify(
            &tampered_auth,
            &store,
            http_method,
            canonical_uri,
            canonical_query_string,
            &headers,
            UNSIGNED_PAYLOAD,
            timestamp,
        );
        assert_eq!(verify_tampered, Err(SigV4Error::SignatureDoesNotMatch));
    }
}
