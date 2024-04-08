use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use tower_sessions::Session;

pub async fn handler(session_manager: Session) -> impl IntoResponse {
    session_manager.remove_value("user_id").await.unwrap();
    (StatusCode::OK, Json(json!({ "status": "OK" })))
}
