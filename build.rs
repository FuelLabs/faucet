use minify_html::Cfg;
use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("index.html");
    let page = include_bytes!("./static/index.html");
    let minified = minify_html::minify(page, &Cfg::spec_compliant());
    fs::write(dest_path, minified).expect("failed to save minified index page");

    println!("cargo:rerun-if-changed=static");
}
