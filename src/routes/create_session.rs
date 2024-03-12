use crate::{
    models::{CreateSessionError, CreateSessionInput, CreateSessionResponse},
    recaptcha,
    session::Salt,
    SharedConfig, SharedSessions,
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Extension, Json,
};
use fuel_types::Address;
use fuels_core::types::bech32::Bech32Address;
use secrecy::ExposeSecret;
use serde_json::json;
use std::{str::FromStr, sync::Arc};

impl IntoResponse for CreateSessionResponse {
    fn into_response(self) -> Response {
        (StatusCode::CREATED, Json(self)).into_response()
    }
}

impl IntoResponse for CreateSessionError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({
                "error": self.error
            })),
        )
            .into_response()
    }
}

pub async fn handler(
    Extension(sessions): Extension<SharedSessions>,
    Extension(pow_difficulty): Extension<Arc<u8>>,
    Extension(config): Extension<SharedConfig>,
    Json(input): Json<CreateSessionInput>,
) -> Result<CreateSessionResponse, CreateSessionError> {
    // parse deposit address
    let address = if let Ok(address) = Address::from_str(input.address.as_str()) {
        Ok(address)
    } else if let Ok(address) = Bech32Address::from_str(input.address.as_str()) {
        Ok(address.into())
    } else {
        return Err(CreateSessionError {
            status: StatusCode::BAD_REQUEST,
            error: "invalid address".to_string(),
        });
    }?;

    // verify captcha
    if let Some(s) = config.captcha_secret.clone() {
        recaptcha::verify(s.expose_secret(), input.captcha.as_str(), None)
            .await
            .map_err(|e| {
                tracing::error!("{}", e);
                CreateSessionError {
                    error: "captcha failed".to_string(),
                    status: StatusCode::UNAUTHORIZED,
                }
            })?;
    }

    let mut map = sessions.lock().await;
    let salt = Salt::random();

    map.insert(salt.clone(), address);

    Ok(CreateSessionResponse {
        status: "Success".to_string(),
        salt: hex::encode(salt.as_bytes()),
        difficulty: *pow_difficulty,
    })
}
