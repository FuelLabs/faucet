use crate::{clerk::ClerkHandler, SharedConfig};
use axum::{http::StatusCode, response::IntoResponse, Extension, Json};
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
    let clerk = ClerkHandler::new(&config);
    let data = clerk.get_user_session(data.value.as_str()).await;
    let data = match data {
        Ok(data) => data,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            )
        }
    };

    let response = Json(json!({
        "user": data.user,
        "session": data.session,
    }));

    session_manager
        .insert("JWT_TOKEN", data.session.id.to_string())
        .await
        .unwrap();

    (StatusCode::OK, response)
}
