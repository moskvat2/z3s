use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Modo de Retenção WORM do S3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetentionMode {
    /// Modo Governance: usuários com permissões especiais podem sobrescrever ou deletar
    #[serde(rename = "GOVERNANCE")]
    Governance,
    /// Modo Compliance: absolutamente ninguém (nem mesmo o root) pode deletar antes do vencimento
    #[serde(rename = "COMPLIANCE")]
    Compliance,
}

/// Configuração de Object Lock do Bucket (GetBucketObjectLockConfiguration)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObjectLockConfiguration {
    pub object_lock_enabled: bool,
    pub rule: Option<ObjectLockRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObjectLockRule {
    pub default_retention: DefaultRetention,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DefaultRetention {
    pub mode: RetentionMode,
    pub days: Option<u32>,
    pub years: Option<u32>,
}

/// Configuração de Retenção de um Objeto específico
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObjectRetention {
    pub mode: RetentionMode,
    pub retain_until_date: DateTime<Utc>,
}

impl ObjectRetention {
    pub fn is_locked(&self) -> bool {
        Utc::now() < self.retain_until_date
    }
}

/// Estado de Legal Hold de um Objeto
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LegalHoldStatus {
    #[serde(rename = "ON")]
    On,
    #[serde(rename = "OFF")]
    Off,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_object_retention_lock() {
        let future_date = Utc::now() + Duration::days(30);
        let past_date = Utc::now() - Duration::days(1);

        let active_lock = ObjectRetention {
            mode: RetentionMode::Compliance,
            retain_until_date: future_date,
        };
        assert!(active_lock.is_locked());

        let expired_lock = ObjectRetention {
            mode: RetentionMode::Compliance,
            retain_until_date: past_date,
        };
        assert!(!expired_lock.is_locked());
    }
}
