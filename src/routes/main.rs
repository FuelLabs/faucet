use crate::SharedConfig;
use axum::{
    extract::Extension,
    response::{Html, IntoResponse},
};
use handlebars::Handlebars;
use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

lazy_static::lazy_static! {
    static ref START_TIME: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
}

fn render_main(public_node_url: String, clerk_pub_key: String) -> String {
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
    // render page
    handlebars.render("index", &data).unwrap()
}

pub async fn handler(Extension(config): Extension<SharedConfig>) -> impl IntoResponse {
    let public_node_url = config.public_node_url.clone();
    let clerk_pub_key = config.clerk_pub_key.clone().unwrap_or("".to_string());
    Html(render_main(public_node_url, clerk_pub_key)).into_response()
}
