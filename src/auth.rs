use axum::async_trait;

pub mod clerk;

#[derive(Debug)]
pub struct AuthError {
    pub message: String,
}

impl AuthError {
    pub fn new(message: impl Into<String>) -> Self {
        AuthError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AuthError {}

type UserId = String;

#[async_trait]
pub trait AuthHandler: Send + Sync {
    async fn get_user_session(&self, session_id: &str) -> Result<UserId, AuthError>;
}
