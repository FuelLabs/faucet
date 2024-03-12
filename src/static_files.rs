use axum::{
    body::Body,
    http::Request,
    middleware::{from_fn, Next},
    response::Response,
    Router,
};
use std::path::Path;
use tower_http::services::ServeDir;

pub async fn content_type_middleware(req: Request<Body>, next: Next) -> Response {
    let uri = req.uri().to_owned();
    let path = uri.path();
    let splited = path.split('.').collect::<Vec<_>>();
    if let Some(extension) = splited.last() {
        let mut response = next.run(req).await;
        let extension = extension.to_owned().to_lowercase();
        let content_type = match extension.as_str() {
            "html" => "text/html",
            "css" => "text/css",
            "js" => "text/javascript",
            "ps" => "application/postscript",
            _ => "application/octet-stream",
        };

        if let Ok(content_type) = content_type.parse() {
            response.headers_mut().insert("Content-Type", content_type);
        }

        response
    } else {
        let mut response = next.run(req).await;

        if let Ok(content_type) = "application/octet-stream".parse() {
            response.headers_mut().insert("Content-Type", content_type);
        }

        response
    }
}

pub fn router<P: AsRef<Path>>(path: P) -> Router {
    let serve_dir = ServeDir::new(path);

    Router::new()
        .fallback_service(serve_dir)
        .layer(from_fn(content_type_middleware))
}
