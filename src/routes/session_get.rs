use axum::{extract::Query, http::StatusCode, response::IntoResponse, Extension, Json};
use hex::FromHexError;
use serde::Deserialize;
use serde_json::json;

use crate::{session::Salt, SharedSessions};

#[derive(Deserialize)]
pub struct SessionQuery {
    salt: String,
}

pub async fn handler(
    query: Query<SessionQuery>,
    Extension(sessions): Extension<SharedSessions>,
) -> impl IntoResponse {
    let salt: Result<[u8; 32], _> = hex::decode(&query.salt).and_then(|value| {
        value
            .try_into()
            .map_err(|_| FromHexError::InvalidStringLength)
    });

    match salt {
        Ok(value) => value,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid salt"})),
            )
                .into_response()
        }
    };

    let map = sessions.lock().await;
    let result = map.get(&Salt::new(salt.unwrap()));

    match result {
        Some(address) => (StatusCode::OK, Json(json!({"address": address}))),
        None => (StatusCode::NOT_FOUND, Json(json!({}))),
    }
    .into_response()
}
