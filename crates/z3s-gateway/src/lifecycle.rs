use crate::xml::LifecycleConfiguration;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use z3s_common::manifest::StorageClass;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LifecycleReport {
    pub expired_objects: usize,
    pub noncurrent_versions_purged: usize,
    pub transitioned_objects: usize,
    pub aborted_uploads: usize,
}

/// Motor de Avaliação e Execução de Regras de Ciclo de Vida S3
pub struct LifecycleEngine {
    configs: Arc<RwLock<HashMap<String, LifecycleConfiguration>>>,
}

impl LifecycleEngine {
    pub fn new() -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_configuration(&self, bucket: &str) -> Option<LifecycleConfiguration> {
        self.configs.read().unwrap().get(bucket).cloned()
    }

    pub fn put_configuration(&self, bucket: &str, config: LifecycleConfiguration) {
        self.configs.write().unwrap().insert(bucket.to_string(), config);
    }

    pub fn delete_configuration(&self, bucket: &str) -> bool {
        self.configs.write().unwrap().remove(bucket).is_some()
    }

    /// Avalia se um objeto deve expirar com base nas regras ativas do bucket
    pub fn should_expire_object(
        &self,
        bucket: &str,
        key: &str,
        created_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> bool {
        let configs = self.configs.read().unwrap();
        let config = match configs.get(bucket) {
            Some(c) => c,
            None => return false,
        };

        for rule in &config.rules {
            if rule.status != "Enabled" {
                continue;
            }

            if let Some(filter) = &rule.filter {
                if let Some(prefix) = &filter.prefix {
                    if !key.starts_with(prefix) {
                        continue;
                    }
                }
            }

            if let Some(exp) = &rule.expiration {
                if let Some(days) = exp.days {
                    let age = now.signed_duration_since(created_at);
                    if age >= Duration::days(days as i64) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Avalia se uma versão não-atual deve ser purgada permanentemente
    pub fn should_purge_noncurrent_version(
        &self,
        bucket: &str,
        key: &str,
        created_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> bool {
        let configs = self.configs.read().unwrap();
        let config = match configs.get(bucket) {
            Some(c) => c,
            None => return false,
        };

        for rule in &config.rules {
            if rule.status != "Enabled" {
                continue;
            }

            if let Some(filter) = &rule.filter {
                if let Some(prefix) = &filter.prefix {
                    if !key.starts_with(prefix) {
                        continue;
                    }
                }
            }

            if let Some(noncurrent_exp) = &rule.noncurrent_version_expiration {
                let age = now.signed_duration_since(created_at);
                if age >= Duration::days(noncurrent_exp.noncurrent_days as i64) {
                    return true;
                }
            }
        }

        false
    }

    /// Avalia se o objeto deve sofrer transição de StorageClass
    pub fn evaluate_transition(
        &self,
        bucket: &str,
        key: &str,
        created_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Option<StorageClass> {
        let configs = self.configs.read().unwrap();
        let config = configs.get(bucket)?;

        for rule in &config.rules {
            if rule.status != "Enabled" {
                continue;
            }

            if let Some(filter) = &rule.filter {
                if let Some(prefix) = &filter.prefix {
                    if !key.starts_with(prefix) {
                        continue;
                    }
                }
            }

            for transition in &rule.transitions {
                if let Some(days) = transition.days {
                    let age = now.signed_duration_since(created_at);
                    if age >= Duration::days(days as i64) {
                        return match transition.storage_class.to_uppercase().as_str() {
                            "STANDARD_IA" => Some(StorageClass::StandardIa),
                            "GLACIER" => Some(StorageClass::Glacier),
                            "REDUCED_REDUNDANCY" => Some(StorageClass::ReducedRedundancy),
                            _ => None,
                        };
                    }
                }
            }
        }

        None
    }

    /// Avalia se um multipart upload incompleto deve ser abortado
    pub fn should_abort_multipart_upload(
        &self,
        bucket: &str,
        key: &str,
        initiated_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> bool {
        let configs = self.configs.read().unwrap();
        let config = match configs.get(bucket) {
            Some(c) => c,
            None => return false,
        };

        for rule in &config.rules {
            if rule.status != "Enabled" {
                continue;
            }

            if let Some(filter) = &rule.filter {
                if let Some(prefix) = &filter.prefix {
                    if !key.starts_with(prefix) {
                        continue;
                    }
                }
            }

            if let Some(abort_rule) = &rule.abort_incomplete_multipart_upload {
                let age = now.signed_duration_since(initiated_at);
                if age >= Duration::days(abort_rule.days_after_initiation as i64) {
                    return true;
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xml::{
        AbortIncompleteMultipartUpload, LifecycleExpiration, LifecycleFilter, LifecycleRule,
        LifecycleTransition, NoncurrentVersionExpiration,
    };

    #[test]
    fn test_lifecycle_rules_evaluation() {
        let engine = LifecycleEngine::new();
        let now = Utc::now();

        let rules = vec![
            LifecycleRule {
                id: "expire-logs".to_string(),
                status: "Enabled".to_string(),
                filter: Some(LifecycleFilter {
                    prefix: Some("logs/".to_string()),
                }),
                expiration: Some(LifecycleExpiration {
                    days: Some(30),
                    date: None,
                    expired_object_delete_marker: None,
                }),
                transitions: vec![LifecycleTransition {
                    days: Some(7),
                    storage_class: "GLACIER".to_string(),
                }],
                noncurrent_version_expiration: Some(NoncurrentVersionExpiration {
                    noncurrent_days: 60,
                    newer_noncurrent_versions: None,
                }),
                abort_incomplete_multipart_upload: Some(AbortIncompleteMultipartUpload {
                    days_after_initiation: 3,
                }),
            },
        ];

        let config = LifecycleConfiguration::new(rules);
        engine.put_configuration("my-bucket", config);

        // 1. Objeto de 5 dias -> Não transiciona nem expira
        let t_5d = now - Duration::days(5);
        assert!(!engine.should_expire_object("my-bucket", "logs/app.log", t_5d, now));
        assert_eq!(engine.evaluate_transition("my-bucket", "logs/app.log", t_5d, now), None);

        // 2. Objeto de 10 dias -> Deve transicionar para GLACIER mas não expirar
        let t_10d = now - Duration::days(10);
        assert!(!engine.should_expire_object("my-bucket", "logs/app.log", t_10d, now));
        assert_eq!(
            engine.evaluate_transition("my-bucket", "logs/app.log", t_10d, now),
            Some(StorageClass::Glacier)
        );

        // 3. Objeto de 35 dias -> Deve expirar
        let t_35d = now - Duration::days(35);
        assert!(engine.should_expire_object("my-bucket", "logs/app.log", t_35d, now));

        // 4. Versão não-atual de 70 dias -> Deve ser purgada
        let t_70d = now - Duration::days(70);
        assert!(engine.should_purge_noncurrent_version("my-bucket", "logs/app.log", t_70d, now));

        // 5. Multipart Upload de 4 dias -> Deve ser abortado
        let t_4d = now - Duration::days(4);
        assert!(engine.should_abort_multipart_upload("my-bucket", "logs/upload.tar", t_4d, now));

        // 6. Objeto fora do prefixo "logs/" -> Não deve sofrer ação
        assert!(!engine.should_expire_object("my-bucket", "data/report.pdf", t_35d, now));
    }
}
