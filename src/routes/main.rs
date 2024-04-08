use crate::SharedConfig;
use axum::{
    extract::Extension,
    response::{Html, IntoResponse},
};
use handlebars::Handlebars;
use std::{
    collections::BTreeMap,
    env,
    error::Error,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

lazy_static::lazy_static! {
    static ref START_TIME: u64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
}

fn get_js_file() -> Result<String, Box<dyn Error>> {
    let project_root = env::var("CARGO_MANIFEST_DIR").expect("Failed to get project root");
    let js_dir = PathBuf::from(project_root).join("static/js");
    let entries = std::fs::read_dir(js_dir)?
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()?;

    let first = entries
        .iter()
        .find(|entry| entry.to_str().unwrap().contains("index"))
        .unwrap();

    let js_file = first.as_path().file_name().unwrap();
    Ok(js_file.to_str().unwrap().to_string())
}

fn render_main(public_node_url: String, clerk_pub_key: String) -> String {
    let js_file = get_js_file().unwrap();
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
    data.insert("js_file", js_file.as_str());
    // render page
    handlebars.render("index", &data).unwrap()
}

pub async fn handler(Extension(config): Extension<SharedConfig>) -> impl IntoResponse {
    let public_node_url = config.public_node_url.clone();
    let clerk_pub_key = config.clerk_pub_key.clone().unwrap_or("".to_string());
    Html(render_main(public_node_url, clerk_pub_key)).into_response()
}
