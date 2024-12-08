use std::{sync::Arc, time::SystemTime};

use async_trait::async_trait;
use aws_config::SdkConfig;
use aws_credential_types::{provider::ProvideCredentials, Credentials};
use futures::lock::Mutex;
use object_store::Error as ObjectStoreError;
use object_store::{aws::AwsCredential, CredentialProvider};

/// AWS Sdk credential provider for object_store
#[derive(Debug)]
#[allow(clippy::type_complexity)]
pub struct AwsCredentialProvider {
    config: SdkConfig,
    cache: Arc<Mutex<Option<(Option<SystemTime>, Credentials)>>>,
}

#[async_trait]
impl CredentialProvider for AwsCredentialProvider {
    type Credential = AwsCredential;

    async fn get_credential(&self) -> Result<Arc<Self::Credential>, ObjectStoreError> {
        let mut guard = self.cache.lock().await;

        let is_valid = if let Some((Some(time), _)) = *guard {
            time >= SystemTime::now()
        } else {
            false
        };

        if !is_valid {
            let provider = self
                .config
                .credentials_provider()
                .ok_or(ObjectStoreError::NotImplemented)?;

            let credentials =
                provider
                    .provide_credentials()
                    .await
                    .map_err(|err| ObjectStoreError::Generic {
                        store: "s3",
                        source: Box::new(err),
                    })?;
            *guard = Some((credentials.expiry(), credentials));
        };

        let credentials = &guard.as_ref().unwrap().1;

        Ok(Arc::new(AwsCredential {
            key_id: credentials.access_key_id().to_string(),
            secret_key: credentials.secret_access_key().to_string(),
            token: credentials.session_token().map(ToString::to_string),
        }))
    }
}

impl AwsCredentialProvider {
    /// Create new credential provider
    pub fn new(config: &SdkConfig) -> Self {
        Self {
            config: config.clone(),
            cache: Arc::new(Mutex::new(None)),
        }
    }
}
