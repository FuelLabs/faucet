use crate::SharedAuthHandler;
use axum::{http::StatusCode, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;
use tower_sessions::Session;

#[derive(Deserialize)]
pub struct SessionData {
    value: String,
}

pub async fn handler(
    Extension(auth_handler): Extension<SharedAuthHandler>,
    session: Session,
    Json(data): Json<SessionData>,
) -> impl IntoResponse {
    let data = auth_handler.get_user_session(data.value.as_str()).await;
    let user_id = match data {
        Ok(user_id) => user_id,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Unauthorized"})),
            )
        }
    };

    let response = Json(json!({
        "user": user_id,
    }));

    session.insert("user_id", user_id).await.unwrap();

    (StatusCode::OK, response)
}
