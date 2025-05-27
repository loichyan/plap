use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=tests");
    let outdir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    generate_includes("tests".as_ref(), &outdir.join("tests_includes.rs"));
}

fn generate_includes(src: &Path, dst: &Path) {
    let mut generated = String::from("// @generated\npub(crate) fn init() {");
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            continue;
        }
        let path = src.join(entry.file_name());
        let contents = fs::read_to_string(path).unwrap();
        let mut lines = contents.lines();
        loop {
            if !lines.any(|l| l.contains("!include")) {
                break;
            }
            generated.push('{');
            for line in &mut lines {
                if line.contains("endinclude!") {
                    break;
                } else {
                    generated.push_str(line);
                    generated.push('\n');
                }
            }
            generated.push('}');
        }
    }
    generated.push('}');

    fs::write(dst, generated).unwrap();
}
