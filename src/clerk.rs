use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config::Config;
use clerk_rs::{
    apis::{sessions_api, users_api},
    clerk::Clerk,
    models::{self, UpdateUserMetadataRequest},
    ClerkConfiguration,
};
use secrecy::ExposeSecret;
use serde_json::json;

#[derive(Debug)]
pub enum ClerkError {
    InvalidSession,
    FailedToGetUser,
    FailedToUpdateUser,
    Other(String), // For other errors, if needed
}

impl std::fmt::Display for ClerkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClerkError::InvalidSession => write!(f, "Invalid session"),
            ClerkError::FailedToGetUser => write!(f, "Failed to get user"),
            ClerkError::FailedToUpdateUser => write!(f, "Failed to update user"),
            ClerkError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ClerkError {}

pub struct ClerkResponse {
    pub user: models::User,
    pub session: models::Session,
}

pub struct ClerkHandler {
    pub client: Clerk,
    pub dispense_limit_interval: u64,
}

impl ClerkHandler {
    // Initialize a new ClerkClientHandler with Clerk configuration
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

    pub async fn update_user_claim(&self, user_id: &str) -> Result<models::User, ClerkError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let user = self.get_user(user_id).await?;
        let update_request = Some(UpdateUserMetadataRequest {
            public_metadata: None,
            unsafe_metadata: None,
            private_metadata: Some(json!({
                "claim_timestamp":  timestamp.as_secs().to_string(),
            })),
        });

        let update_res = users_api::User::update_user_metadata(
            &self.client,
            user.clone().id.unwrap().as_str(),
            update_request,
        )
        .await;

        if update_res.is_err() {
            return Err(ClerkError::FailedToUpdateUser);
        }

        Ok(user)
    }

    pub fn check_dispense_interval(&self, seconds_str: &str) -> bool {
        if let Ok(seconds) = u64::from_str_radix(seconds_str, 10) {
            let given_time = UNIX_EPOCH + Duration::from_secs(seconds);

            if let Ok(duration_since_given) = SystemTime::now().duration_since(given_time) {
                let diff_seconds = duration_since_given.as_secs();
                return diff_seconds >= self.dispense_limit_interval
                    && diff_seconds < self.dispense_limit_interval * 2;
            }
        }
        false
    }

    pub async fn check_user_claim(&self, user_id: &str) -> Result<bool, ClerkError> {
        let user = self.get_user(user_id).await?;
        match user.private_metadata {
            Some(metadata) => {
                let value = metadata.unwrap();
                let claim_timestamp = value["claim_timestamp"].as_str().unwrap_or("0");
                return Ok(self.check_dispense_interval(claim_timestamp));
            }
            None => Ok(false),
        }
    }

    pub async fn get_user(&self, user_id: &str) -> Result<models::User, ClerkError> {
        let user_res = users_api::User::get_user(&self.client, user_id).await;
        let user = if let Ok(user_res) = user_res {
            user_res
        } else {
            return Err(ClerkError::FailedToGetUser);
        };
        Ok(user)
    }

    pub async fn get_session(&self, session_token: &str) -> Result<models::Session, ClerkError> {
        let session_res = sessions_api::Session::get_session(&self.client, session_token).await;
        let session = if let Ok(session_res) = session_res {
            session_res
        } else {
            return Err(ClerkError::InvalidSession);
        };
        Ok(session)
    }

    // Retrieve session and user information
    pub async fn get_user_session(&self, session_token: &str) -> Result<ClerkResponse, ClerkError> {
        let session = self.get_session(session_token).await?;
        let user_id = session.user_id.clone();
        let user = self.get_user(user_id.as_str()).await?;
        Ok(ClerkResponse { user, session })
    }

    pub async fn user_id_from_session(&self, session_token: &str) -> Result<String, ClerkError> {
        let session = self.get_session(session_token).await?;
        let user = self.get_user(session.user_id.as_str()).await?;
        Ok(user.id.unwrap())
    }
}
