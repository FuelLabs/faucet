use crate::SharedConfig;
use axum::{http::StatusCode, response::IntoResponse, Extension, Json};
use clerk_rs::{apis::sessions_api, clerk::Clerk, ClerkConfiguration};
use secrecy::ExposeSecret;
use serde::Deserialize;
use serde_json::json;
use tower_sessions::Session;

#[derive(Deserialize)]
pub struct SessionData {
    value: String,
}

pub async fn handler(
    Extension(config): Extension<SharedConfig>,
    session_manager: Session,
    Json(data): Json<SessionData>,
) -> impl IntoResponse {
    let clerk_secret_key = config.clerk_secret_key.clone().unwrap();
    let clerk_key = Some(clerk_secret_key.expose_secret().clone());
    let clerk_config = ClerkConfiguration::new(None, None, clerk_key, None);
    let client = Clerk::new(clerk_config);
    let res = sessions_api::Session::get_session(&client, data.value.as_str()).await;
    let item = match res {
        Ok(session) => {
            session_manager
                .insert("JWT_TOKEN", session.id.to_string())
                .await
                .unwrap();
            (StatusCode::OK, Json(json!(session)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    };
    item.into_response()
}
