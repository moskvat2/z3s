use serde::{Deserialize, Serialize};

/// Decisão de avaliação de política IAM / Bucket Policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// Ação de política S3 (ex: "s3:GetObject", "s3:PutObject", "s3:*")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PolicyAction {
    Single(String),
    Multiple(Vec<String>),
}

impl PolicyAction {
    pub fn matches(&self, action: &str) -> bool {
        match self {
            PolicyAction::Single(a) => Self::matches_pattern(a, action),
            PolicyAction::Multiple(actions) => actions.iter().any(|a| Self::matches_pattern(a, action)),
        }
    }

    fn matches_pattern(pattern: &str, action: &str) -> bool {
        if pattern == "*" || pattern == "s3:*" {
            return true;
        }
        if let Some(prefix) = pattern.strip_suffix('*') {
            return action.starts_with(prefix);
        }
        pattern.eq_ignore_ascii_case(action)
    }
}

/// Principal (usuário ou wildcard "*")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PolicyPrincipal {
    Wildcard(String),
    Aws {
        #[serde(rename = "AWS")]
        aws: serde_json::Value,
    },
}

impl PolicyPrincipal {
    pub fn matches(&self, principal_arn: &str) -> bool {
        match self {
            PolicyPrincipal::Wildcard(s) => s == "*",
            PolicyPrincipal::Aws { aws } => {
                if let Some(s) = aws.as_str() {
                    s == "*" || s == principal_arn
                } else if let Some(arr) = aws.as_array() {
                    arr.iter().any(|v| v.as_str() == Some("*") || v.as_str() == Some(principal_arn))
                } else {
                    false
                }
            }
        }
    }
}

/// Resource ARN (ex: "arn:aws:s3:::bucket/*")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PolicyResource {
    Single(String),
    Multiple(Vec<String>),
}

impl PolicyResource {
    pub fn matches(&self, resource_arn: &str) -> bool {
        match self {
            PolicyResource::Single(r) => Self::matches_pattern(r, resource_arn),
            PolicyResource::Multiple(resources) => {
                resources.iter().any(|r| Self::matches_pattern(r, resource_arn))
            }
        }
    }

    fn matches_pattern(pattern: &str, resource: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if let Some(prefix) = pattern.strip_suffix('*') {
            return resource.starts_with(prefix);
        }
        pattern == resource
    }
}

/// Declaração individual em uma política S3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statement {
    #[serde(rename = "Sid", skip_serializing_if = "Option::is_none")]
    pub sid: Option<String>,
    #[serde(rename = "Effect")]
    pub effect: String, // "Allow" ou "Deny"
    #[serde(rename = "Principal")]
    pub principal: PolicyPrincipal,
    #[serde(rename = "Action")]
    pub action: PolicyAction,
    #[serde(rename = "Resource")]
    pub resource: PolicyResource,
}

/// Documento JSON de Bucket Policy padrão AWS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketPolicy {
    #[serde(rename = "Version", default = "default_version")]
    pub version: String,
    #[serde(rename = "Statement")]
    pub statement: Vec<Statement>,
}

fn default_version() -> String {
    "2012-10-17".to_string()
}

impl BucketPolicy {
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    /// Avalia a política para um usuário, ação e recurso.
    /// Retorna:
    /// - Some(PolicyEffect::Deny) se houver qualquer Deny explícito.
    /// - Some(PolicyEffect::Allow) se houver Allow e nenhum Deny.
    /// - None se não houver correspondência (Default Deny implícito).
    pub fn evaluate(&self, principal_arn: &str, action: &str, resource_arn: &str) -> Option<PolicyEffect> {
        let mut has_allow = false;

        for stmt in &self.statement {
            if stmt.principal.matches(principal_arn)
                && stmt.action.matches(action)
                && stmt.resource.matches(resource_arn)
            {
                if stmt.effect.eq_ignore_ascii_case("Deny") {
                    return Some(PolicyEffect::Deny); // Explicit Deny sempre tem precedência absoluta
                } else if stmt.effect.eq_ignore_ascii_case("Allow") {
                    has_allow = true;
                }
            }
        }

        if has_allow {
            Some(PolicyEffect::Allow)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bucket_policy_evaluation() {
        let policy_json = r#"{
            "Version": "2012-10-17",
            "Statement": [
                {
                    "Sid": "PublicReadOnly",
                    "Effect": "Allow",
                    "Principal": "*",
                    "Action": ["s3:GetObject"],
                    "Resource": "arn:aws:s3:::public-bucket/*"
                },
                {
                    "Sid": "DenySecretKey",
                    "Effect": "Deny",
                    "Principal": "*",
                    "Action": "s3:*",
                    "Resource": "arn:aws:s3:::public-bucket/secret/*"
                }
            ]
        }"#;

        let policy = BucketPolicy::from_json(policy_json).unwrap();

        // 1. GetObject em arquivo público -> Allow
        assert_eq!(
            policy.evaluate("arn:aws:iam::123:user/alice", "s3:GetObject", "arn:aws:s3:::public-bucket/logo.png"),
            Some(PolicyEffect::Allow)
        );

        // 2. PutObject em arquivo público -> None (não permitido)
        assert_eq!(
            policy.evaluate("arn:aws:iam::123:user/alice", "s3:PutObject", "arn:aws:s3:::public-bucket/logo.png"),
            None
        );

        // 3. GetObject em pasta secreta -> Deny explícito
        assert_eq!(
            policy.evaluate("arn:aws:iam::123:user/alice", "s3:GetObject", "arn:aws:s3:::public-bucket/secret/keys.txt"),
            Some(PolicyEffect::Deny)
        );
    }
}
