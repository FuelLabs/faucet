use axum::response::{IntoResponse, Redirect};

pub async fn handler() -> impl IntoResponse {
    Redirect::permanent("/static/img/favicon.svg").into_response()
}
