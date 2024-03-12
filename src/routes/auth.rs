use std::collections::BTreeMap;

use axum::{
    response::{Html, IntoResponse, Redirect},
    Extension,
};
use handlebars::Handlebars;
use tower_sessions::Session;

use crate::SharedConfig;

fn render_auth(clerk_pub_key: String) -> String {
    let template = include_str!(concat!(env!("OUT_DIR"), "/index.html"));
    // sub in values
    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string("index", template)
        .unwrap();
    let mut data = BTreeMap::new();
    data.insert("clerk_public_key", clerk_pub_key.as_str());
    data.insert("page", "auth");
    // render page
    handlebars.render("index", &data).unwrap()
}

pub async fn handler(
    Extension(config): Extension<SharedConfig>,
    session: Session,
) -> impl IntoResponse {
    let clerk_pub_key = config.clerk_pub_key.clone();
    let jwt_token: Option<String> = session.get("JWT_TOKEN").await.unwrap();

    match jwt_token {
        Some(_) => Redirect::temporary("/").into_response(),
        None => Html(render_auth(clerk_pub_key.unwrap_or("".to_string()))).into_response(),
    }
}
