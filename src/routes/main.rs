use crate::SharedConfig;
use axum::{
    extract::{Extension, Query},
    response::{Html, IntoResponse, Redirect},
};
use handlebars::Handlebars;
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};
use tower_sessions::Session;

lazy_static::lazy_static! {
    static ref START_TIME: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
}

fn render_main(
    public_node_url: String,
    captcha_key: Option<String>,
    clerk_pub_key: String,
) -> String {
    let template = include_str!(concat!(env!("OUT_DIR"), "/index.html"));
    // sub in values
    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string("index", template)
        .unwrap();
    let mut data = BTreeMap::new();
    data.insert("page_title", "Fuel Faucet");
    data.insert("public_node_url", public_node_url.as_str());
    data.insert("clerk_public_key", clerk_pub_key.as_str());
    // if captcha is enabled, add captcha key
    if let Some(captcha_key) = &captcha_key {
        data.insert("captcha_key", captcha_key.as_str());
    }
    data.insert("page", "faucet");
    // render page
    handlebars.render("index", &data).unwrap()
}

#[derive(Deserialize, Debug)]
pub struct Method {
    pub method: Option<String>,
}

pub async fn handler(
    Extension(config): Extension<SharedConfig>,
    session: Session,
    method: Query<Method>,
) -> impl IntoResponse {
    let public_node_url = config.public_node_url.clone();
    let captcha_key = config.captcha_key.clone();
    let clerk_pub_key = config.clerk_pub_key.clone().unwrap_or("".to_string());
    let jwt_token: Option<String> = session.get("JWT_TOKEN").await.unwrap();

    let redirect_to_auth = || Redirect::temporary("/auth").into_response();
    let html_response =
        || Html(render_main(public_node_url, captcha_key, clerk_pub_key)).into_response();

    if jwt_token.is_some() {
        return html_response();
    }

    let value = method.method.as_ref();
    match value {
        None => redirect_to_auth(),
        Some(value) => match value.as_str() {
            "pow" => html_response(),
            _ => redirect_to_auth(),
        },
    }
}
