use crate::config::Config;
use axum::async_trait;
use clerk_rs::{apis::sessions_api, clerk::Clerk, models, ClerkConfiguration};
use secrecy::ExposeSecret;

use super::{AuthError, AuthHandler};

pub struct ClerkHandler {
    pub client: Clerk,
    pub dispense_limit_interval: u64,
}

impl ClerkHandler {
    pub fn new(config: &Config) -> Self {
        let clerk_secret_key = config
            .clerk_secret_key
            .clone()
            .expect("Clerk secret key is required");
        let clerk_key = Some(clerk_secret_key.expose_secret().clone());
        let clerk_config = ClerkConfiguration::new(None, None, clerk_key, None);
        let client = Clerk::new(clerk_config);
        ClerkHandler {
            client,
            dispense_limit_interval: config.dispense_limit_interval,
        }
    }

    async fn get_session(&self, session_id: &str) -> Result<models::Session, AuthError> {
        let session = sessions_api::Session::get_session(&self.client, session_id)
            .await
            .map_err(|_| AuthError::new("Failed to retrieve session".to_string()))?;

        Ok(session)
    }
}

#[async_trait]
impl AuthHandler for ClerkHandler {
    async fn get_user_session(&self, session_id: &str) -> Result<String, AuthError> {
        let session = self.get_session(session_id).await?;
        Ok(session.user_id)
    }
}
