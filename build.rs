use minify_html::Cfg;
use std::{env, fs, path::Path};

fn build(page: &str, raw: &[u8]) {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join(page);
    let minified = minify_html::minify(raw, &Cfg::spec_compliant());
    fs::write(dest_path, minified).expect("failed to save minified index page");
    let html_dest_path = Path::new(&out_dir).join("index.html");
    let page = include_bytes!("./static/index.html");
    let minified = minify_html::minify(page, &Cfg::spec_compliant());
    fs::write(html_dest_path, minified).expect("failed to save minified index page");

    let worker_dest_path = Path::new(&out_dir).join("worker.js");
    let worker_script = include_bytes!("./static/worker.js");
    fs::write(worker_dest_path, worker_script).expect("failed to save worker script");

    println!("cargo:rerun-if-changed=static");
}

fn main() {
    build("index.html", include_bytes!("./static/index.html"));
}
