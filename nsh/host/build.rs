//! Compiles the app's `locales/*.yml` into the JSON the host embeds.

use std::{collections::BTreeMap, env, fs, path::PathBuf};

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../locales");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut locales: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for entry in fs::read_dir(&dir).expect("locales") {
        let path = entry.expect("entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("yml") {
            continue;
        }
        let language = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("language")
            .to_owned();
        let text = fs::read_to_string(&path).expect("locale");
        let value: serde_yaml::Value = serde_yaml::from_str(&text).expect("locale yaml");
        let mut flat = BTreeMap::new();
        flatten(&value, "", &mut flat);
        locales.insert(language, flat);
    }
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR")).join("translations.json");
    fs::write(out, serde_json::to_string(&locales).expect("json")).expect("write");
}

fn flatten(value: &serde_yaml::Value, prefix: &str, out: &mut BTreeMap<String, String>) {
    match value {
        serde_yaml::Value::Mapping(map) => {
            for (key, value) in map {
                let Some(key) = key.as_str() else { continue };
                let key = if prefix.is_empty() {
                    key.to_owned()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten(value, &key, out);
            }
        }
        serde_yaml::Value::String(text) => {
            out.insert(prefix.to_owned(), text.clone());
        }
        serde_yaml::Value::Number(number) => {
            out.insert(prefix.to_owned(), number.to_string());
        }
        _ => {}
    }
}
